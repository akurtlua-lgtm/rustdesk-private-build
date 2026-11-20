use super::*;
use crate::server::mobile_host::Registration;
use hbb_common::tokio::task::JoinSet;

const MAX_CONNECTIONS: usize = 8;

// The application owns the runtime and awaits this worker after dropping its capture authority.
pub(crate) async fn run(registration: Registration, host: String) -> ResultType<()> {
    if Config::is_proxy() || use_ws() || crate::is_udp_disabled() {
        bail!("Mobile sharing currently requires direct UDP registration and TCP relay access");
    }
    if config::is_outgoing_only()
        || option2bool("stop-service", &Config::get_option("stop-service"))
    {
        bail!("Incoming mobile sharing is disabled");
    }
    let mut work = JoinSet::new();
    let mut stop = registration.stop_receiver();
    while !registration.stopped() {
        if config::is_outgoing_only()
            || option2bool("stop-service", &Config::get_option("stop-service"))
        {
            registration.revoke();
            break;
        }
        let result = register_once(&registration, &host, &mut work).await;
        registration.set_registered(false);
        if registration.stopped() {
            break;
        }
        if let Err(error) = result {
            hbb_common::throttled_log!(
                PUNCH_LOG_INTERVAL,
                warn,
                "Mobile registration failed: {}",
                error
            );
        }
        if stop
            .until(tokio::time::sleep(Duration::from_secs(3)))
            .await
            .is_none()
        {
            break;
        }
    }
    while let Some(result) = work.join_next().await {
        report_work(result);
    }
    Ok(())
}

fn report_work(result: Result<ResultType<()>, tokio::task::JoinError>) {
    if !matches!(result, Ok(Ok(()))) {
        hbb_common::throttled_log!(
            PUNCH_LOG_INTERVAL,
            warn,
            "Mobile connection worker ended with an error"
        );
    }
}

async fn register_once(
    registration: &Registration,
    host: &str,
    work: &mut JoinSet<ResultType<()>>,
) -> ResultType<()> {
    let mut stop = registration.stop_receiver();
    let host = check_port(host, RENDEZVOUS_PORT);
    let (mut socket, addr) = stop
        .until(new_udp_for(&host, CONNECT_TIMEOUT))
        .await
        .ok_or_else(|| anyhow::anyhow!("Capture stopped"))??;
    let mut rz = RendezvousMediator {
        addr: addr.clone(),
        host: host.clone(),
        host_prefix: RendezvousMediator::get_host_prefix(&host),
        keep_alive: crate::DEFAULT_KEEP_ALIVE,
    };
    stop.until(rz.register_pk(Sink::Framed(&mut socket, &addr)))
        .await
        .ok_or_else(|| anyhow::anyhow!("Capture stopped"))??;
    let mut last_recv = Instant::now();
    let mut last_sent = Instant::now();
    let mut confirmed = false;
    let mut timer = interval(Duration::from_secs(1));
    loop {
        select! {
            biased;
            _ = stop.recv() => break,
            completed = work.join_next(), if !work.is_empty() => {
                if let Some(result) = completed { report_work(result); }
            }
            received = socket.next() => {
                let (bytes, source) = received.ok_or_else(|| anyhow::anyhow!("Registration socket closed"))??;
                if source != addr { continue; }
                last_recv = Instant::now();
                let message = Message::parse_from_bytes(&bytes)?;
                match message.union {
                    Some(rendezvous_message::Union::RegisterPkResponse(response)) => {
                        if response.result.enum_value() != Ok(register_pk_response::Result::OK) {
                            bail!("Mobile identity registration was refused");
                        }
                        confirmed = true;
                        let mut peer = Message::new();
                        peer.set_register_peer(RegisterPeer { id: Config::get_id(), serial: Config::get_serial(), ..Default::default() });
                        stop.until(socket.send(&peer, addr.clone())).await
                            .ok_or_else(|| anyhow::anyhow!("Capture stopped"))??;
                        last_sent = Instant::now();
                    }
                    Some(rendezvous_message::Union::RegisterPeerResponse(response)) => {
                        if response.request_pk {
                            confirmed = false;
                            registration.set_registered(false);
                            stop.until(rz.register_pk(Sink::Framed(&mut socket, &addr))).await
                                .ok_or_else(|| anyhow::anyhow!("Capture stopped"))??;
                            last_sent = Instant::now();
                        } else if confirmed {
                            registration.set_registered(true);
                        }
                    }
                    Some(rendezvous_message::Union::PunchHole(ph)) if confirmed && work.len() < MAX_CONNECTIONS => {
                        let relay = rz.get_relay_server(ph.relay_server);
                        remember_reported_relay(&relay, Instant::now());
                        let mut request = RequestRelay { socket_addr: ph.socket_addr, relay_server: relay,
                            uuid: Uuid::new_v4().to_string(), secure: true, ..Default::default() };
                        request.control_permissions = ph.control_permissions;
                        request.controlled_context = ph.controlled_context;
                        work.spawn(relay_session(registration.clone(), rz.clone(), request, true));
                    }
                    Some(rendezvous_message::Union::RequestRelay(request)) if confirmed && work.len() < MAX_CONNECTIONS => {
                        rz.check_relay_request(&request, Instant::now())?;
                        if !request.secure { bail!("Mobile sharing requires encryption"); }
                        work.spawn(relay_session(registration.clone(), rz.clone(), request, false));
                    }
                    Some(rendezvous_message::Union::FetchLocalAddr(local)) if confirmed && work.len() < MAX_CONNECTIONS => {
                        let relay = rz.get_relay_server(local.relay_server);
                        remember_reported_relay(&relay, Instant::now());
                        let request = RequestRelay { socket_addr: local.socket_addr, relay_server: relay,
                            uuid: Uuid::new_v4().to_string(), secure: true,
                            control_permissions: local.control_permissions,
                            controlled_context: local.controlled_context, ..Default::default() };
                        work.spawn(relay_session(registration.clone(), rz.clone(), request, true));
                    }
                    _ => {}
                }
            }
            _ = timer.tick() => {
                if last_recv.elapsed().as_millis() > (REG_INTERVAL as u128) * 3 {
                    bail!("Mobile registration timed out");
                }
                if option2bool("stop-service", &Config::get_option("stop-service")) || config::is_outgoing_only() {
                    registration.revoke();
                    bail!("Incoming mobile sharing was disabled");
                }
                let delay = if confirmed { REG_INTERVAL as u128 } else { 3000 };
                if last_sent.elapsed().as_millis() >= delay {
                    let sent = async {
                        if confirmed {
                            let mut message = Message::new();
                            message.set_register_peer(RegisterPeer { id: Config::get_id(), serial: Config::get_serial(), ..Default::default() });
                            socket.send(&message, addr.clone()).await
                        } else {
                            rz.register_pk(Sink::Framed(&mut socket, &addr)).await
                        }
                    };
                    stop.until(sent).await.ok_or_else(|| anyhow::anyhow!("Capture stopped"))??;
                    last_sent = Instant::now();
                }
            }
        }
    }
    Ok(())
}

async fn relay_session(
    registration: Registration,
    rz: RendezvousMediator,
    request: RequestRelay,
    initiate: bool,
) -> ResultType<()> {
    let mut stopped = registration.stop_receiver();
    let setup = async {
        let mut rendezvous = connect_tcp(&*rz.host, CONNECT_TIMEOUT).await?;
        let key = crate::get_key(true).await;
        let mut response = RelayResponse {
            socket_addr: request.socket_addr.clone(),
            version: crate::VERSION.into(),
            ..Default::default()
        };
        if initiate {
            response.uuid = request.uuid.clone();
            response.relay_server = request.relay_server.clone();
            response.set_id(Config::get_id());
        }
        let mut reply = Message::new();
        reply.set_relay_response(response);
        rendezvous.send(&reply).await?;
        let relay = check_port(&request.relay_server, config::RELAY_PORT);
        let mut stream = connect_tcp(relay, CONNECT_TIMEOUT).await?;
        let mut join = Message::new();
        join.set_request_relay(RequestRelay {
            licence_key: key,
            uuid: request.uuid.clone(),
            ..Default::default()
        });
        stream.send(&join).await?;
        Ok::<_, anyhow::Error>(stream)
    };
    let Some(stream) = stopped.until(setup).await else {
        return Ok(());
    };
    // The established connection owns revocation and runs its normal unsubscribe/cache cleanup.
    crate::server::create_tcp_connection(
        registration.server(),
        stream?,
        AddrMangle::decode(&request.socket_addr),
        true,
        connection_meta(
            request.control_permissions.into_option(),
            request.controlled_context.into_option(),
        ),
    )
    .await
}
