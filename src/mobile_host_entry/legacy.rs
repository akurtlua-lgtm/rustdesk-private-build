//! Private, device-local Keychain bootstrap for the iOS 15–26 broadcast process.
use super::{write_output, CURRENT};
use crate::auth_2fa::TOTPInfo;
use hbb_common::{config::Config, password_security::decrypt_vec_or_original};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) static EXTENSION_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: u8,
    totp: Option<TOTPInfo>,
}

/// # Safety
/// The writable output contract matches the other native getters. This is a secret snapshot:
/// keep it only in device-local shared Keychain, never app-group files, logs or account requests.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_legacy_policy(output: *mut u8, capacity: usize) -> usize {
    if hbb_common::config::is_outgoing_only()
        || hbb_common::config::option2bool("stop-service", &Config::get_option("stop-service"))
        || Config::is_proxy()
        || crate::is_udp_disabled()
        || hbb_common::config::use_ws()
    {
        return 0;
    }
    let _current = CURRENT.lock().unwrap();
    let stored = Config::get_option("2fa");
    let totp = if stored.is_empty() {
        None
    } else {
        if TOTPInfo::from_str(&stored).is_err() {
            return 0;
        }
        let Ok(mut info) = serde_json::from_str::<TOTPInfo>(&stored) else {
            return 0;
        };
        let (secret, success, _) = decrypt_vec_or_original(&info.secret, "00");
        if !success {
            return 0;
        }
        info.secret = secret;
        Some(info)
    };
    let Ok(bytes) = serde_json::to_vec(&Policy { schema: 1, totp }) else {
        return 0;
    };
    // SAFETY: forwarding the documented writable capacity, no pointer retained.
    unsafe { write_output(&bytes, output, capacity) }
}

/// # Safety
/// The input is borrowed for the call and comes only from device-local shared Keychain.
/// Import is restricted to a freshly initialized extension process before a sharing scope exists.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_legacy_import(data: *const u8, length: usize) -> i32 {
    if !EXTENSION_INITIALIZED.load(Ordering::Acquire) || data.is_null() || length > 16384 {
        return -1;
    }
    let current = CURRENT.lock().unwrap();
    if current.is_some() {
        return -2;
    }
    // SAFETY: the caller owns readable policy bytes for this synchronous validation.
    let bytes = unsafe { std::slice::from_raw_parts(data, length) };
    let Ok(policy) = serde_json::from_slice::<Policy>(bytes) else {
        return -1;
    };
    if policy.schema != 1 {
        return -1;
    }
    let encoded = match policy.totp {
        None => String::new(),
        Some(info) => {
            let Ok(encoded) = info.into_string() else {
                return -1;
            };
            if TOTPInfo::from_str(&encoded).is_err() {
                return -1;
            }
            encoded
        }
    };
    if crate::ui_interface::set_auth_option("2fa".into(), encoded).is_err() {
        return -1;
    }
    0
}
