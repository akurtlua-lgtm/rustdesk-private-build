use hbb_common::sodiumoxide::crypto::onetimeauth::poly1305;

#[test]
fn initialized_entries_preserve_authentication() {
    const CHILD: &str = "RUSTDESK_SODIUM_COMPAT_CHILD";
    if let Ok(entry) = std::env::var(CHILD) {
        if entry == "core" {
            assert!(crate::common::global_init());
        } else {
            assert_eq!(entry, "flutter");
            let dir = std::env::temp_dir().join(format!("rustdesk-sodium-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("test directory");
            super::initialize(dir.to_str().expect("test directory UTF-8"), "");
        }
        let key = poly1305::Key([7; 32]);
        let message = [42; 100];
        let mut state =
            std::mem::MaybeUninit::<libsodium_sys::crypto_onetimeauth_poly1305_state>::uninit();
        // SAFETY: libsodium initializes the correctly aligned state; the key has its required length.
        assert_eq!(
            unsafe {
                libsodium_sys::crypto_onetimeauth_poly1305_init(state.as_mut_ptr(), key.0.as_ptr())
            },
            0
        );
        // SAFETY: state was initialized successfully, and the input slice has the supplied length.
        assert_eq!(
            unsafe {
                libsodium_sys::crypto_onetimeauth_poly1305_update(
                    state.as_mut_ptr(),
                    message.as_ptr(),
                    50,
                )
            },
            0
        );
        // The actual update verifier could previously initialize sodium during a live computation.
        crate::update_metadata::verify_private_update_metadata(
            include_bytes!("update_metadata/fixtures/private-release.json"),
            include_bytes!("update_metadata/fixtures/private-release.json.sig"),
            "1.5.8",
            crate::update_metadata::UpdateArtifactQuery {
                platform: "macos",
                arch: "aarch64",
                format: "dmg",
            },
        )
        .expect("existing signed update fixture");
        let mut tag = [0; 16];
        // SAFETY: initialized state, valid remaining input and correctly sized output buffer.
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_onetimeauth_poly1305_update(
                    state.as_mut_ptr(),
                    message[50..].as_ptr(),
                    50
                ),
                0
            );
            assert_eq!(
                libsodium_sys::crypto_onetimeauth_poly1305_final(
                    state.as_mut_ptr(),
                    tag.as_mut_ptr()
                ),
                0
            );
        }
        assert_eq!(
            tag.as_slice(),
            poly1305::authenticate(&message, &key).as_ref()
        );
        return;
    }
    for entry in ["core", "flutter"] {
        let result = std::process::Command::new(
            std::env::current_exe().expect("native test executable"),
        )
        .args([
            "--exact",
            "flutter_ffi::sodium_startup_tests::initialized_entries_preserve_authentication",
            "--nocapture",
        ])
        .env(CHILD, entry)
        .output()
        .expect("fresh native test process");
        assert!(
            result.status.success(),
            "{entry} startup failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
