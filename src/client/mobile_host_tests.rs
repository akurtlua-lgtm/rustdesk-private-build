use super::*;
use base::message_proto::{
    login_response, misc, option_message, supported_decoding, Auth2FA, KeyEvent, OptionMessage,
    SupportedDecoding,
};
use hbb_common::{
    config::{APP_DIR, APP_HOME_DIR},
    sha2::{Digest, Sha256},
    tcp::FramedStream,
};
use std::sync::{Arc, Once};

const PASSWORD: &str = "synthetic-mobile-test-password";
static CONFIG: Once = Once::new();

fn initialize() {
    CONFIG.call_once(|| {
        let directory =
            std::env::temp_dir().join(format!("rd-mobile-host-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        *APP_DIR.write().unwrap() = directory.to_str().unwrap().to_owned();
        *APP_HOME_DIR.write().unwrap() = directory.to_str().unwrap().to_owned();
        crate::common::init_sodium();
        Config::set_id("123456789");
        Config::set_option(
            "verification-method".into(),
            "use-permanent-password".into(),
        );
        Config::set_option("approve-mode".into(), "password".into());
        assert!(Config::set_permanent_password(PASSWORD));
        let info = crate::auth_2fa::TOTPInfo {
            name: "synthetic-mobile-host".into(),
            secret: vec![17; 20],
            digits: 6,
            created_at: 0,
        };
        Config::set_option("2fa".into(), info.into_string().unwrap());
    });
}

fn scope() -> crate::server::mobile_host::ScopedHost {
    let inbox = Arc::new(scrap::mobile_frame::FrameInbox::default());
    inbox
        .publish_bgra(&vec![128; 128 * 96 * 4], 128, 96, 128 * 4)
        .unwrap();
    crate::server::mobile_host::prepare(inbox).unwrap()
}

async fn incoming(
    scope: &crate::server::mobile_host::ScopedHost,
    secure: bool,
) -> (Stream, tokio::task::JoinHandle<ResultType<()>>) {
    let listener = hbb_common::tcp::new_listener("127.0.0.1:0", false)
        .await
        .unwrap();
    let mut controller = connect_tcp(listener.local_addr().unwrap().to_string(), 3000)
        .await
        .unwrap();
    controller.set_send_timeout(3000);
    let (accepted, address) = listener.accept().await.unwrap();
    let server = scope.server();
    let task = tokio::spawn(crate::server::create_tcp_connection(
        server,
        Stream::Tcp(FramedStream::from(accepted, address)),
        address,
        secure,
        Default::default(),
    ));
    (controller, task)
}

async fn read(stream: &mut Stream) -> Message {
    let bytes = timeout(5000, stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    Message::parse_from_bytes(&bytes).unwrap()
}

async fn closed(stream: &mut Stream) {
    timeout(5000, async {
        while let Some(result) = stream.next().await {
            assert!(
                result.is_ok(),
                "socket must reach EOF without a framing/decryption error"
            );
        }
    })
    .await
    .expect("capture scope must close the live socket");
}

async fn encrypted(stream: &mut Stream) {
    let (hub_pk, hub_sk) = sign::gen_keypair();
    let signed = sign::sign(
        &IdPk {
            id: Config::get_id(),
            pk: Config::get_key_pair().1.into(),
            ..Default::default()
        }
        .write_to_bytes()
        .unwrap(),
        &hub_sk,
    );
    let key = Client::secure_connection(
        &Config::get_id(),
        signed,
        &crate::encode64(hub_pk.0),
        stream,
    )
    .await
    .unwrap();
    assert_eq!(key, Some(Config::get_key_pair().1));
    assert!(stream.is_secured());
}

async fn login_response(stream: &mut Stream) -> (login_response::Union, usize) {
    let mut permissions = std::collections::BTreeSet::new();
    timeout(5000, async {
        loop {
            let message = read(stream).await;
            match message.union {
                Some(message::Union::LoginResponse(response)) => {
                    return (response.union.unwrap(), permissions.len())
                }
                Some(message::Union::Misc(value)) => {
                    if let Some(misc::Union::PermissionInfo(permission)) = value.union {
                        assert!(!permission.enabled);
                        permissions.insert(permission.permission.value());
                    }
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}

async fn login(stream: &mut Stream, session: u64) {
    let hash = loop {
        if let Some(message::Union::Hash(hash)) = read(stream).await.union {
            break hash;
        }
    };
    let mut first = Sha256::new();
    first.update(PASSWORD);
    first.update(hash.salt.as_bytes());
    let mut second = Sha256::new();
    second.update(first.finalize());
    second.update(hash.challenge.as_bytes());
    let mut request = Message::new();
    request.set_login_request(LoginRequest {
        username: Config::get_id(),
        password: second.finalize().to_vec().into(),
        my_id: format!("synthetic-controller-{session}"),
        my_name: "synthetic-controller".into(),
        version: crate::VERSION.into(),
        my_platform: "Windows".into(),
        session_id: session,
        option: Some(OptionMessage {
            supported_decoding: Some(SupportedDecoding {
                ability_vp8: 1,
                prefer: supported_decoding::PreferCodec::VP8.into(),
                ..Default::default()
            })
            .into(),
            disable_audio: option_message::BoolOption::Yes.into(),
            ..Default::default()
        })
        .into(),
        ..Default::default()
    });
    stream.send(&request).await.unwrap();
    let (response, permissions) = login_response(stream).await;
    assert_eq!(permissions, 8);
    assert!(matches!(response, login_response::Union::Error(error) if error == REQUIRE_2FA));
    let totp = crate::auth_2fa::TOTPInfo::from_str(&Config::get_option("2fa")).unwrap();
    let mut response = Message::new();
    response.set_auth_2fa(Auth2FA {
        code: totp.generate_current().unwrap(),
        trust_this_device: false,
        ..Default::default()
    });
    stream.send(&response).await.unwrap();
    let (response, _) = login_response(stream).await;
    let login_response::Union::PeerInfo(peer) = response else {
        panic!("encrypted password and TOTP login must authorize");
    };
    assert_eq!(peer.platform, "iOS");
    assert_eq!(peer.displays.len(), 1);
    assert_eq!((peer.displays[0].width, peer.displays[0].height), (128, 96));
}

#[tokio::test]
async fn encrypted_password_totp_read_only_and_scope_revocation() {
    initialize();
    let scope = scope();
    let (mut rejected, rejected_task) = incoming(&scope, true).await;
    encrypted(&mut rejected).await;
    login(&mut rejected, 101).await;
    timeout(5000, async {
        loop {
            if let Some(message::Union::VideoFrame(frame)) = read(&mut rejected).await.union {
                assert!(matches!(frame.union, Some(base::message_proto::video_frame::Union::Vp8s(frames)) if !frames.frames.is_empty()));
                break;
            }
        }
    }).await.unwrap();
    let mut key = Message::new();
    key.set_key_event(KeyEvent::new());
    rejected.send(&key).await.unwrap();
    closed(&mut rejected).await;
    timeout(5000, rejected_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let (mut active, active_task) = incoming(&scope, true).await;
    encrypted(&mut active).await;
    login(&mut active, 102).await;
    let mut ping = Message::new();
    ping.set_test_delay(TestDelay {
        time: 5151,
        from_client: true,
        ..Default::default()
    });
    active.send(&ping).await.unwrap();
    timeout(5000, async {
        loop {
            if let Some(message::Union::TestDelay(delay)) = read(&mut active).await.union {
                if delay.from_client && delay.time == 5151 {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    drop(scope);
    closed(&mut active).await;
    timeout(5000, active_task).await.unwrap().unwrap().unwrap();
}

#[tokio::test]
async fn plaintext_refused_and_stalled_handshake_revoked() {
    initialize();
    let scope = scope();
    let (mut plaintext, task) = incoming(&scope, false).await;
    assert!(timeout(5000, task).await.unwrap().unwrap().is_err());
    closed(&mut plaintext).await;
    let (mut fallback, fallback_task) = incoming(&scope, true).await;
    assert!(matches!(
        read(&mut fallback).await.union,
        Some(message::Union::SignedId(_))
    ));
    fallback.send(&Message::new()).await.unwrap();
    assert!(timeout(5000, fallback_task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    closed(&mut fallback).await;
    let (mut stalled, task) = incoming(&scope, true).await;
    assert!(matches!(
        read(&mut stalled).await.union,
        Some(message::Union::SignedId(_))
    ));
    drop(scope);
    assert!(timeout(5000, task).await.unwrap().unwrap().is_err());
    closed(&mut stalled).await;
}

#[tokio::test]
async fn registration_ack_relay_key_and_inflight_work_revoke_together() {
    use hbb_common::rendezvous_proto::{
        PunchHole, RegisterPeerResponse, RegisterPkResponse, RendezvousMessage,
    };
    initialize();
    let scope = scope();
    let registration = scope.registration();
    let listener = hbb_common::tcp::new_listener("127.0.0.1:0", false)
        .await
        .unwrap();
    let relay = hbb_common::tcp::new_listener("127.0.0.1:0", false)
        .await
        .unwrap();
    let udp = hbb_common::tokio::net::UdpSocket::bind(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (public, _) = sign::gen_keypair();
    let key = crate::encode64(public.0);
    let saved_key = Config::get_option("key");
    let saved_relay = Config::get_option("relay-server");
    Config::set_option("key".into(), key.clone());
    Config::set_option(
        "relay-server".into(),
        relay.local_addr().unwrap().to_string(),
    );
    assert!(!scope.is_registered());
    let task = tokio::spawn(crate::rendezvous_mediator::mobile_sharing::run(
        registration,
        listener.local_addr().unwrap().to_string(),
    ));
    let mut bytes = [0u8; 4096];
    let (length, device) = timeout(5000, udp.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let registered = RendezvousMessage::parse_from_bytes(&bytes[..length]).unwrap();
    let Some(hbb_common::rendezvous_proto::rendezvous_message::Union::RegisterPk(registered)) =
        registered.union
    else {
        panic!("native OSS UDP RegisterPk required");
    };
    assert_eq!(registered.id, Config::get_id());
    assert_eq!(registered.pk.as_ref(), Config::get_key_pair().1);
    assert!(!scope.is_registered());
    let mut reply = RendezvousMessage::new();
    reply.set_register_pk_response(RegisterPkResponse::new());
    udp.send_to(&reply.write_to_bytes().unwrap(), device)
        .await
        .unwrap();
    let (length, source) = timeout(5000, udp.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(source, device);
    let peer = RendezvousMessage::parse_from_bytes(&bytes[..length]).unwrap();
    let Some(hbb_common::rendezvous_proto::rendezvous_message::Union::RegisterPeer(peer)) =
        peer.union
    else {
        panic!("current UDP socket must be registered before readiness");
    };
    assert_eq!(peer.id, Config::get_id());
    assert!(!scope.is_registered());
    reply.set_register_peer_response(RegisterPeerResponse::new());
    udp.send_to(&reply.write_to_bytes().unwrap(), device)
        .await
        .unwrap();
    timeout(5000, async {
        while !scope.is_registered() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut punch = RendezvousMessage::new();
    punch.set_punch_hole(PunchHole {
        socket_addr: hbb_common::AddrMangle::encode("127.0.0.1:12345".parse().unwrap()).into(),
        relay_server: relay.local_addr().unwrap().to_string(),
        ..Default::default()
    });
    udp.send_to(&punch.write_to_bytes().unwrap(), device)
        .await
        .unwrap();
    let response = timeout(5000, async {
        loop {
            let (incoming, address) = listener.accept().await.unwrap();
            let mut response = FramedStream::from(incoming, address);
            // The native UDP resolver can open an empty TCP reachability probe first.
            if let Some(bytes) = response.next_timeout(1000).await {
                break RendezvousMessage::parse_from_bytes(&bytes.unwrap()).unwrap();
            }
        }
    })
    .await
    .unwrap();
    let Some(hbb_common::rendezvous_proto::rendezvous_message::Union::RelayResponse(response)) =
        response.union
    else {
        panic!("OSS TCP RelayResponse required");
    };
    let uuid = &response.uuid;
    uuid::Uuid::parse_str(uuid).unwrap();
    let (incoming, address) = timeout(5000, relay.accept()).await.unwrap().unwrap();
    let mut controller = Stream::Tcp(FramedStream::from(incoming, address));
    let bytes = timeout(5000, controller.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let join = RendezvousMessage::parse_from_bytes(&bytes).unwrap();
    let Some(hbb_common::rendezvous_proto::rendezvous_message::Union::RequestRelay(join)) =
        join.union
    else {
        panic!("relay join required");
    };
    assert_eq!(join.licence_key, key);
    assert_eq!(&join.uuid, uuid);
    assert!(matches!(
        read(&mut controller).await.union,
        Some(message::Union::SignedId(_))
    ));
    drop(scope);
    closed(&mut controller).await;
    timeout(5000, task).await.unwrap().unwrap().unwrap();
    drop(self::scope());
    Config::set_option("key".into(), saved_key);
    Config::set_option("relay-server".into(), saved_relay);
}

#[tokio::test]
async fn native_entry_owns_only_one_runtime_and_revokes_stale_callbacks() {
    use crate::mobile_host_entry::*;
    use hbb_common::rendezvous_proto::{
        rendezvous_message, RegisterPeerResponse, RegisterPkResponse, RendezvousMessage,
    };
    initialize();
    let udp = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let saved_host = Config::get_option("custom-rendezvous-server");
    let saved_key = Config::get_option("key");
    Config::set_option(
        "custom-rendezvous-server".into(),
        udp.local_addr().unwrap().to_string(),
    );
    let saved_relay = Config::get_option("relay-server");
    let (public, _) = sign::gen_keypair();
    let servers = serde_json::json!({"schema":1,"id_server":udp.local_addr().unwrap().to_string(),
        "relay_server":"","key":crate::encode64(public.0)});
    let encoded = serde_json::to_vec(&servers).unwrap();
    assert_eq!(
        unsafe { rd_mobile_host_configure_servers(encoded.as_ptr(), encoded.len()) },
        0
    );
    let mut forbidden = servers.clone();
    forbidden["access_token"] = serde_json::json!("rdoss_v1_synthetic");
    let encoded = serde_json::to_vec(&forbidden).unwrap();
    assert_eq!(
        unsafe { rd_mobile_host_configure_servers(encoded.as_ptr(), encoded.len()) },
        -1
    );
    let needed = unsafe { rd_mobile_host_servers(std::ptr::null_mut(), 0) };
    let mut snapshot = vec![0; needed];
    assert_eq!(
        unsafe { rd_mobile_host_servers(snapshot.as_mut_ptr(), snapshot.len()) },
        needed
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&snapshot[..needed - 1]).unwrap(),
        servers
    );
    let pixels = vec![128; 128 * 96 * 4];
    assert_eq!(
        unsafe { rd_mobile_host_begin(std::ptr::null(), pixels.len(), 128, 96, 512) },
        0
    );
    let token = unsafe { rd_mobile_host_begin(pixels.as_ptr(), pixels.len(), 128, 96, 512) };
    assert!(token > 0);
    assert_eq!(rd_mobile_host_state(token), 1);
    assert_eq!(
        unsafe { rd_mobile_host_begin(pixels.as_ptr(), pixels.len(), 128, 96, 512) },
        0
    );
    assert_eq!(
        rd_mobile_host_run(token),
        -2,
        "existing runtime must not be nested"
    );
    let worker = std::thread::spawn(move || rd_mobile_host_run(token));
    let mut bytes = [0u8; 4096];
    let (length, device) = timeout(5000, udp.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let request = RendezvousMessage::parse_from_bytes(&bytes[..length]).unwrap();
    assert!(matches!(
        request.union,
        Some(rendezvous_message::Union::RegisterPk(_))
    ));
    let mut reply = RendezvousMessage::new();
    reply.set_register_pk_response(RegisterPkResponse::new());
    udp.send_to(&reply.write_to_bytes().unwrap(), device)
        .await
        .unwrap();
    let (length, source) = timeout(5000, udp.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(source, device);
    let request = RendezvousMessage::parse_from_bytes(&bytes[..length]).unwrap();
    assert!(matches!(
        request.union,
        Some(rendezvous_message::Union::RegisterPeer(_))
    ));
    assert_eq!(rd_mobile_host_state(token), 2);
    reply.set_register_peer_response(RegisterPeerResponse::new());
    udp.send_to(&reply.write_to_bytes().unwrap(), device)
        .await
        .unwrap();
    timeout(5000, async {
        while rd_mobile_host_state(token) != 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        unsafe { rd_mobile_host_frame(token, pixels.as_ptr(), pixels.len(), 128, 96, 512) },
        0
    );
    assert_eq!(rd_mobile_host_stop(token), 0);
    timeout(5000, async {
        while !worker.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(worker.join().unwrap(), 0);
    assert_eq!(rd_mobile_host_state(token), 5);
    assert_eq!(rd_mobile_host_release(token), 0);
    assert_eq!(rd_mobile_host_state(token), 0);
    let next = unsafe { rd_mobile_host_begin(pixels.as_ptr(), pixels.len(), 128, 96, 512) };
    assert!(next > token);
    assert_eq!(
        unsafe { rd_mobile_host_frame(token, pixels.as_ptr(), pixels.len(), 128, 96, 512) },
        -1
    );
    assert_eq!(rd_mobile_host_state(next), 1);
    assert_eq!(rd_mobile_host_stop(next), 0);
    assert_eq!(rd_mobile_host_release(next), 0);
    Config::set_option("custom-rendezvous-server".into(), saved_host);
    Config::set_option("key".into(), saved_key);
    Config::set_option("relay-server".into(), saved_relay);
}
