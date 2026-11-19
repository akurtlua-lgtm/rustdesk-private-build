fn crypto_roundtrip() -> bool {
    use hbb_common::sodiumoxide::{self, crypto::box_};

    if sodiumoxide::init().is_err() {
        return false;
    }
    let (sender_public, sender_secret) = box_::gen_keypair();
    let (receiver_public, receiver_secret) = box_::gen_keypair();
    let nonce = box_::gen_nonce();
    let message = b"RustDesk extension link probe";
    let cipher = box_::seal(message, &nonce, &receiver_public, &sender_secret);
    box_::open(&cipher, &nonce, &sender_public, &receiver_secret)
        .is_ok_and(|plain| plain == message)
}

#[cfg(all(target_os = "ios", feature = "ios-host-link-probe"))]
#[no_mangle]
pub extern "C" fn rd_ios_host_link_probe() -> i32 {
    i32::from(crypto_roundtrip())
}

#[cfg(test)]
mod tests {
    #[test]
    fn extension_probe_initializes_authenticated_crypto() {
        assert!(super::crypto_roundtrip());
    }
}
