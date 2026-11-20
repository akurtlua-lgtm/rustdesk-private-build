//! Mobile application process entry. Call `rd_mobile_host_run` on a dedicated native thread,
//! never a Tokio worker. Frame callbacks and stop never create runtimes or block on networking.

use crate::server::mobile_host::{self, ScopedHost};
use hbb_common::{
    config::Config,
    password_security::{self, ApproveMode},
    tokio,
};
use scrap::mobile_frame::FrameInbox;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

static CURRENT: Mutex<Option<Arc<Session>>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);
static INITIALIZE: Mutex<()> = Mutex::new(());

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SharedServers {
    schema: u8,
    id_server: String,
    relay_server: String,
    key: String,
}

fn valid_server(server: &str, port: i32) -> bool {
    if server.is_empty() || server.len() > 2048 {
        return false;
    }
    let Ok(url) = url::Url::parse(&format!("tcp://{}", crate::check_port(server, port))) else {
        return false;
    };
    url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.path(), "" | "/")
}

/// # Safety
/// `directory` is a valid UTF-8 byte slice for the call, naming an existing app-owned directory.
/// The extension calls this before any Config access; an initialized app cannot change directories.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_initialize(directory: *const u8, length: usize) -> i32 {
    if directory.is_null() || length == 0 || length > 4096 {
        return -1;
    }
    // SAFETY: the caller owns the readable UTF-8 bytes until return.
    let Ok(directory) =
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(directory, length) })
    else {
        return -1;
    };
    let path = Path::new(directory);
    if !path.is_absolute() || !path.is_dir() {
        return -1;
    }
    let _initialize = INITIALIZE.lock().unwrap();
    crate::common::init_sodium();
    let existing = hbb_common::config::APP_DIR.read().unwrap().clone();
    if !existing.is_empty() {
        return if Path::new(&existing) == path { 0 } else { -1 };
    }
    *hbb_common::config::APP_DIR.write().unwrap() = directory.to_owned();
    *hbb_common::config::APP_HOME_DIR.write().unwrap() = directory.to_owned();
    crate::load_custom_client();
    0
}

/// # Safety
/// `password` contains valid readable UTF-8 for this call. The local user explicitly requested
/// this password-only sharing configuration; no account credentials are accepted or recorded.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_set_password(password: *const u8, length: usize) -> i32 {
    if password.is_null() || length > 128 {
        return -1;
    }
    // SAFETY: bytes are borrowed only for the synchronous password configuration.
    let Ok(password) = std::str::from_utf8(unsafe { std::slice::from_raw_parts(password, length) })
    else {
        return -1;
    };
    if password.chars().count() < 8 {
        return -1;
    }
    let current = CURRENT.lock().unwrap();
    if current.is_some() {
        return -2;
    }
    crate::common::init_sodium();
    if !crate::ui_interface::set_permanent_password_with_result(password.to_owned()) {
        return -1;
    }
    Config::set_option(
        "verification-method".into(),
        "use-permanent-password".into(),
    );
    Config::set_option("approve-mode".into(), "password".into());
    0
}

/// # Safety
/// `data` is a readable UTF-8 JSON slice. Used only to copy the user's server choices to their
/// own extension; the schema deliberately has no account token, password or identity-private-key.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_configure_servers(data: *const u8, length: usize) -> i32 {
    if data.is_null() || length == 0 || length > 16384 {
        return -1;
    }
    // SAFETY: JSON is fully parsed before the borrowed slice can expire.
    let Ok(settings) = serde_json::from_slice::<SharedServers>(unsafe {
        std::slice::from_raw_parts(data, length)
    }) else {
        return -1;
    };
    if settings.schema != 1
        || !valid_server(&settings.id_server, hbb_common::config::RENDEZVOUS_PORT)
        || (!settings.relay_server.is_empty()
            && !valid_server(&settings.relay_server, hbb_common::config::RELAY_PORT))
        || crate::get_rs_pk(&settings.key).is_none()
    {
        return -1;
    }
    let current = CURRENT.lock().unwrap();
    if current.is_some() {
        return -2;
    }
    Config::set_option("custom-rendezvous-server".into(), settings.id_server);
    Config::set_option("relay-server".into(), settings.relay_server);
    Config::set_option("key".into(), settings.key);
    0
}

unsafe fn write_output(bytes: &[u8], output: *mut u8, capacity: usize) -> usize {
    let required = bytes.len() + 1;
    if !output.is_null() && capacity >= required {
        // SAFETY: the public getters require writable capacity and retain no pointer.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
            output.add(bytes.len()).write(0);
        }
    }
    required
}

/// # Safety
/// `output` is null or writable for `capacity` bytes. Returns required size including NUL;
/// when too small, it writes nothing. The private snapshot contains only server choices.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_servers(output: *mut u8, capacity: usize) -> usize {
    let _current = CURRENT.lock().unwrap();
    let settings = SharedServers {
        schema: 1,
        id_server: Config::get_rendezvous_server(),
        relay_server: Config::get_option("relay-server"),
        key: Config::get_option("key"),
    };
    let Ok(bytes) = serde_json::to_vec(&settings) else {
        return 0;
    };
    // SAFETY: forwarded output contract is the same as write_output's writable-capacity contract.
    unsafe { write_output(&bytes, output, capacity) }
}

/// # Safety
/// `output` is null or writable for `capacity` bytes. Returns required size including NUL.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_id(output: *mut u8, capacity: usize) -> usize {
    let id = Config::get_id();
    // SAFETY: forwarded output contract is the same as write_output's writable-capacity contract.
    unsafe { write_output(id.as_bytes(), output, capacity) }
}

struct Session {
    token: u64,
    source: Arc<FrameInbox>,
    authority: Mutex<Option<ScopedHost>>,
    started: AtomicBool,
    finished: AtomicBool,
    result: AtomicI32,
}

impl Session {
    fn close(&self) {
        self.source.close();
        let authority = self.authority.lock().unwrap().take();
        drop(authority);
    }
}

fn session(token: u64) -> Option<Arc<Session>> {
    CURRENT
        .lock()
        .unwrap()
        .as_ref()
        .filter(|s| s.token == token)
        .cloned()
}

/// # Safety
/// `pixels` is readable for `length` bytes for this call. The caller owns system capture consent;
/// pass only a real, owned packed BGRA frame from its currently authorized callback.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_begin(
    pixels: *const u8,
    length: usize,
    width: u32,
    height: u32,
    stride: usize,
) -> u64 {
    crate::common::init_sodium();
    if pixels.is_null()
        || length > 32 * 1024 * 1024
        || password_security::approve_mode() != ApproveMode::Password
        || Config::get_option("verification-method") != "use-permanent-password"
        || !password_security::permanent_enabled()
        || !Config::has_permanent_password()
        || crate::get_rs_pk(&Config::get_option("key")).is_none()
    {
        return 0;
    }
    let mut current = CURRENT.lock().unwrap();
    if current.is_some() {
        return 0;
    }
    let source = Arc::new(FrameInbox::default());
    // SAFETY: the caller's readable buffer contract is limited to this synchronous copy.
    let bytes = unsafe { std::slice::from_raw_parts(pixels, length) };
    if source
        .publish_bgra(bytes, width as usize, height as usize, stride)
        .is_err()
    {
        return 0;
    }
    let Ok(authority) = mobile_host::prepare(source.clone()) else {
        return 0;
    };
    let Ok(token) = NEXT.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
    else {
        return 0;
    };
    *current = Some(Arc::new(Session {
        token,
        source,
        authority: Mutex::new(Some(authority)),
        started: AtomicBool::new(false),
        finished: AtomicBool::new(false),
        result: AtomicI32::new(0),
    }));
    token
}

/// # Safety
/// `pixels` is readable for `length` bytes for this synchronous call; no borrowed bytes are retained.
#[no_mangle]
pub unsafe extern "C" fn rd_mobile_host_frame(
    token: u64,
    pixels: *const u8,
    length: usize,
    width: u32,
    height: u32,
    stride: usize,
) -> i32 {
    if pixels.is_null() || length > 32 * 1024 * 1024 {
        return -1;
    }
    let Some(session) = session(token) else {
        return -1;
    };
    // SAFETY: the caller's readable buffer contract holds until publish finishes copying.
    let bytes = unsafe { std::slice::from_raw_parts(pixels, length) };
    match session
        .source
        .publish_bgra(bytes, width as usize, height as usize, stride)
    {
        Ok(()) => 0,
        Err(_) => {
            session.close();
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn rd_mobile_host_stop(token: u64) -> i32 {
    let Some(session) = session(token) else {
        return -1;
    };
    session.close();
    0
}

// 0: absent, 1: waiting for native worker, 2: registering, 3: ready, 4: stopping, 5: stopped, 6: failed.
#[no_mangle]
pub extern "C" fn rd_mobile_host_state(token: u64) -> i32 {
    let Some(session) = session(token) else {
        return 0;
    };
    if session.finished.load(Ordering::Acquire) {
        return if session.result.load(Ordering::Acquire) == 0 {
            5
        } else {
            6
        };
    }
    if session.source.dimensions().is_none() {
        return 4;
    }
    if !session.started.load(Ordering::Acquire) {
        return 1;
    }
    if session
        .authority
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(ScopedHost::is_registered)
    {
        3
    } else {
        2
    }
}

/// The application's one runtime owner for this sharing process. Never call on the UI thread.
/// The owner must stop capture, call stop, await this function, then release the token.
#[no_mangle]
pub extern "C" fn rd_mobile_host_run(token: u64) -> i32 {
    if tokio::runtime::Handle::try_current().is_ok() {
        return -2;
    }
    let Some(session) = session(token) else {
        return -1;
    };
    if session
        .started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return -1;
    }
    let registration = session
        .authority
        .lock()
        .unwrap()
        .as_ref()
        .map(ScopedHost::registration);
    let result = if let Some(registration) = registration {
        if registration.stopped() {
            0
        } else {
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => {
                    let host = Config::get_rendezvous_server();
                    match runtime.block_on(crate::rendezvous_mediator::mobile_sharing::run(
                        registration,
                        host,
                    )) {
                        Ok(()) => 0,
                        Err(_) => -3,
                    }
                }
                Err(_) => -4,
            }
        }
    } else {
        0
    };
    session.close();
    session.result.store(result, Ordering::Release);
    session.finished.store(true, Ordering::Release);
    result
}

#[no_mangle]
pub extern "C" fn rd_mobile_host_release(token: u64) -> i32 {
    let Some(session) = session(token) else {
        return -1;
    };
    session.close();
    if session.started.load(Ordering::Acquire) && !session.finished.load(Ordering::Acquire) {
        return -2;
    }
    let mut current = CURRENT.lock().unwrap();
    if current
        .as_ref()
        .is_some_and(|current| current.token == token)
    {
        *current = None;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_before_configuration_and_set_password() {
        let directory =
            std::env::temp_dir().join(format!("mobile-native-init-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let name = directory.to_str().unwrap().as_bytes();
        assert_eq!(
            unsafe { rd_mobile_host_initialize(name.as_ptr(), name.len()) },
            0
        );
        assert_eq!(
            unsafe { rd_mobile_host_initialize(name.as_ptr(), name.len()) },
            0
        );
        let other = std::env::temp_dir();
        let other = other.to_str().unwrap().as_bytes();
        assert_eq!(
            unsafe { rd_mobile_host_initialize(other.as_ptr(), other.len()) },
            -1
        );
        assert_eq!(
            unsafe { rd_mobile_host_set_password(b"short".as_ptr(), 5) },
            -1
        );
        let password = b"synthetic-native-device-password";
        assert_eq!(
            unsafe { rd_mobile_host_set_password(password.as_ptr(), password.len()) },
            0
        );
        assert!(Config::has_permanent_password());
        assert_eq!(password_security::approve_mode(), ApproveMode::Password);
        assert_eq!(
            Config::get_option("verification-method"),
            "use-permanent-password"
        );
    }
}
