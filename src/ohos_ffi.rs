//! Controller-only C ABI. String arguments are borrowed NUL-terminated UTF-8 for the call.
//! Returned strings belong to Rust and must be released only with `rd_ohos_free_string`.
//! Callbacks must not re-enter this module. Unregistering waits for borrowed callbacks to return.

use crate::{
    client::{Interface, QualityStatus},
    ui_session_interface::{io_loop, InvokeUiSession, Session},
};
use base::message_proto::*;
use hbb_common::{
    base64::{engine::general_purpose::STANDARD, Engine},
    config::{self, Config},
    rendezvous_proto::ConnType,
    SessionID,
};
use scrap::ImageRgb;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    ffi::{c_char, CStr, CString},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock, RwLock,
    },
    time::Duration,
};
use uuid::Uuid;

type EventCallback = unsafe extern "C" fn(*const u8, usize) -> i32;
type RgbaCallback =
    unsafe extern "C" fn(*const c_char, u32, *const u8, usize, u32, u32, usize) -> i32;

#[derive(Clone, Copy)]
struct Callbacks {
    event: Option<EventCallback>,
    rgba: Option<RgbaCallback>,
}

static CALLBACKS: RwLock<Callbacks> = RwLock::new(Callbacks {
    event: None,
    rgba: None,
});
static INITIALIZED: OnceLock<()> = OnceLock::new();

hbb_common::lazy_static::lazy_static! {
    static ref SESSIONS: RwLock<HashMap<SessionID, Arc<Session<OhosHandler>>>> = RwLock::new(HashMap::new());
}

#[derive(Clone, Default)]
pub struct OhosHandler {
    id: SessionID,
    first_frame: Arc<AtomicBool>,
}

impl OhosHandler {
    fn emit(&self, name: &str, mut value: Value) {
        value["name"] = json!(name);
        value["sid"] = json!(self.id.to_string());
        match serde_json::to_vec(&value) {
            Ok(data) => {
                let callbacks = CALLBACKS.read().unwrap();
                let status = callbacks
                    .event
                    .map(|callback| unsafe { callback(data.as_ptr(), data.len()) });
                drop(callbacks);
                if let Some(status) = status.filter(|status| *status != 0) {
                    hbb_common::throttled_log!(
                        Duration::from_secs(10),
                        warn,
                        "OHOS event delivery failed: {status}"
                    );
                }
            }
            Err(error) => hbb_common::throttled_log!(
                Duration::from_secs(10),
                warn,
                "OHOS event encoding failed: {error}"
            ),
        }
    }
}

fn displays(displays: &[DisplayInfo]) -> Value {
    json!(displays.iter().map(|d| json!({"x":d.x,"y":d.y,"width":d.width,"height":d.height,"cursor_embedded":d.cursor_embedded})).collect::<Vec<_>>())
}

impl InvokeUiSession for OhosHandler {
    fn set_cursor_data(&self, data: CursorData) {
        self.emit("cursor_data", json!({"id":data.id.to_string(),"hotx":data.hotx,"hoty":data.hoty,"width":data.width,"height":data.height,"colors":STANDARD.encode(data.colors)}));
    }
    fn set_cursor_id(&self, id: String) {
        self.emit("cursor_id", json!({"id":id}));
    }
    fn set_cursor_position(&self, pos: CursorPosition) {
        self.emit("cursor_position", json!({"x":pos.x,"y":pos.y}));
    }
    fn set_display(&self, x: i32, y: i32, w: i32, h: i32, cursor_embedded: bool, scale: f64) {
        self.emit("display",json!({"x":x,"y":y,"width":w,"height":h,"cursor_embedded":cursor_embedded,"scale":scale}));
    }
    fn switch_display(&self, display: &SwitchDisplay) {
        self.emit("switch_display",json!({"display":display.display,"x":display.x,"y":display.y,"width":display.width,"height":display.height}));
    }
    fn set_peer_info(&self, info: &PeerInfo) {
        self.emit("peer_info",json!({"platform":info.platform,"version":info.version,"current_display":info.current_display,"displays":displays(&info.displays)}));
    }
    fn set_displays(&self, info: &Vec<DisplayInfo>) {
        self.emit("displays", json!({"displays":displays(info)}));
    }
    fn set_platform_additions(&self, data: &str) {
        self.emit("platform_additions", json!({"data":data}));
    }
    fn on_connected(&self, _kind: ConnType) {
        self.emit("authenticated", json!({}));
    }
    fn update_privacy_mode(&self) {
        self.emit("privacy_mode", json!({}));
    }
    fn set_permission(&self, name: &str, value: bool) {
        self.emit("permission", json!({"permission":name,"enabled":value}));
    }
    fn close_success(&self) {
        self.emit("closed", json!({}));
    }
    fn update_quality_status(&self, status: QualityStatus) {
        self.emit("quality",json!({"speed":status.speed,"fps":status.fps,"delay":status.delay,"codec":status.codec_format.map(|value|value.to_string())}));
    }
    fn set_connection_type(&self, secure: bool, direct: bool, kind: &str) {
        self.emit(
            "transport_ready",
            json!({"secure":secure,"direct":direct,"transport":kind}),
        );
    }
    fn set_fingerprint(&self, value: String) {
        self.emit("peer_fingerprint", json!({"value":value}));
    }
    fn job_error(&self, id: i32, error: String, file: i32) {
        self.emit("job_error", json!({"id":id,"error":error,"file":file}));
    }
    fn job_done(&self, id: i32, file: i32) {
        self.emit("job_done", json!({"id":id,"file":file}));
    }
    fn clear_all_jobs(&self) {
        self.emit("clear_jobs", json!({}));
    }
    fn new_message(&self, text: String) {
        self.emit("chat", json!({"text":text}));
    }
    fn update_transfer_list(&self) {
        self.emit("transfer_list", json!({}));
    }
    fn load_last_job(&self, count: i32, data: &str, auto_start: bool) {
        self.emit(
            "last_job",
            json!({"count":count,"data":data,"auto_start":auto_start}),
        );
    }
    fn update_folder_files(
        &self,
        id: i32,
        _entries: &Vec<FileEntry>,
        path: String,
        local: bool,
        count_only: bool,
    ) {
        self.emit(
            "folder_files",
            json!({"id":id,"path":path,"local":local,"count_only":count_only}),
        );
    }
    fn confirm_delete_files(&self, id: i32, index: i32, name: String) {
        self.emit("confirm_delete", json!({"id":id,"index":index,"file":name}));
    }
    fn override_file_confirm(
        &self,
        id: i32,
        file: i32,
        path: String,
        upload: bool,
        identical: bool,
    ) {
        self.emit(
            "confirm_override",
            json!({"id":id,"file":file,"path":path,"upload":upload,"identical":identical}),
        );
    }
    fn update_block_input_state(&self, on: bool) {
        self.emit("block_input", json!({"enabled":on}));
    }
    fn job_progress(&self, id: i32, file: i32, speed: f64, size: f64) {
        self.emit(
            "job_progress",
            json!({"id":id,"file":file,"speed":speed,"size":size}),
        );
    }
    fn adapt_size(&self) {
        self.emit("adapt_size", json!({}));
    }
    fn on_rgba(&self, display: usize, rgba: &mut ImageRgb) {
        let callbacks = CALLBACKS.read().unwrap();
        let Some(callback) = callbacks.rgba else {
            return;
        };
        let Some(row) = rgba.w.checked_mul(4) else {
            return;
        };
        if rgba.h == 0 || rgba.raw.len() % rgba.h != 0 || rgba.raw.len() / rgba.h < row {
            return;
        }
        let (Ok(width), Ok(height), Ok(display)) = (
            u32::try_from(rgba.w),
            u32::try_from(rgba.h),
            u32::try_from(display),
        ) else {
            return;
        };
        let Ok(id) = CString::new(self.id.to_string()) else {
            return;
        };
        let result = unsafe {
            callback(
                id.as_ptr(),
                display,
                rgba.raw.as_ptr(),
                rgba.raw.len(),
                width,
                height,
                rgba.raw.len() / rgba.h,
            )
        };
        drop(callbacks);
        if result == 0 && !self.first_frame.swap(true, Ordering::AcqRel) {
            self.emit("first_frame", json!({}));
        }
    }
    fn msgbox(&self, kind: &str, title: &str, text: &str, link: &str, retry: bool) {
        self.emit(
            "message",
            json!({"type":kind,"title":title,"text":text,"link":link,"retry":retry}),
        );
    }
    fn clipboard(&self, text: String) {
        self.emit("clipboard", json!({"text":text}));
    }
    fn cancel_msgbox(&self, tag: &str) {
        self.emit("cancel_message", json!({"tag":tag}));
    }
    fn switch_back(&self, id: &str) {
        self.emit("switch_back", json!({"id":id}));
    }
    fn portable_service_running(&self, running: bool) {
        self.emit("portable_service", json!({"running":running}));
    }
    fn on_voice_call_started(&self) {
        self.emit("voice_started", json!({}));
    }
    fn on_voice_call_closed(&self, reason: &str) {
        self.emit("voice_closed", json!({"reason":reason}));
    }
    fn on_voice_call_waiting(&self) {
        self.emit("voice_waiting", json!({}));
    }
    fn on_voice_call_incoming(&self) {
        self.emit("voice_incoming", json!({}));
    }
    fn get_rgba(&self, _display: usize) -> *const u8 {
        std::ptr::null()
    }
    fn next_rgba(&self, _display: usize) {}
    fn set_multiple_windows_session(&self, _sessions: Vec<WindowsSession>) {
        self.emit("windows_sessions", json!({}));
    }
    fn set_current_display(&self, display: i32) {
        self.emit("current_display", json!({"display":display}));
    }
    fn update_record_status(&self, running: bool) {
        self.emit("recording", json!({"running":running}));
    }
    fn printer_request(&self, id: i32, path: String) {
        self.emit("printer_request", json!({"id":id,"path":path}));
    }
    fn handle_screenshot_resp(&self, id: String, message: String) {
        self.emit("screenshot", json!({"id":id,"message":message}));
    }
    fn handle_terminal_response(&self, _response: TerminalResponse) {
        self.emit("unsupported_terminal", json!({}));
    }
}

unsafe fn argument<'a>(value: *const c_char) -> Result<&'a str, ()> {
    if value.is_null() {
        return Err(());
    }
    unsafe { CStr::from_ptr(value) }.to_str().map_err(|_| ())
}

unsafe fn session(value: *const c_char) -> Option<Arc<Session<OhosHandler>>> {
    let id = Uuid::parse_str(unsafe { argument(value) }.ok()?).ok()?;
    SESSIONS.read().unwrap().get(&id).cloned()
}

fn owned(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn can_control(session: &Session<OhosHandler>) -> bool {
    if !session.ui_handler.first_frame.load(Ordering::Acquire)
        || !*session.server_keyboard_enabled.read().unwrap()
        || !session
            .connection_round_state
            .lock()
            .unwrap()
            .is_connected()
    {
        return false;
    }
    let config = session.lc.read().unwrap();
    config.peer_info.is_some() && !config.view_only.v
}

pub(crate) fn is_session_active(config: &Arc<RwLock<crate::client::LoginConfigHandler>>) -> bool {
    SESSIONS
        .read()
        .unwrap()
        .values()
        .any(|session| Arc::ptr_eq(&session.lc, config))
}

/// # Safety
/// `home` and `files` follow the module's C string contract and name app-owned absolute directories.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_init(home: *const c_char, files: *const c_char) -> i32 {
    if INITIALIZED.get().is_some() {
        return 0;
    }
    let (Ok(home), Ok(files)) = (unsafe { argument(home) }, unsafe { argument(files) }) else {
        return -1;
    };
    if !Path::new(home).is_absolute()
        || !Path::new(files).is_absolute()
        || !Path::new(home).is_dir()
        || !Path::new(files).is_dir()
    {
        return -1;
    }
    if hbb_common::sodiumoxide::init().is_err() {
        return -2;
    }
    *config::APP_HOME_DIR.write().unwrap() = home.to_owned();
    *config::APP_DIR.write().unwrap() = files.to_owned();
    crate::load_custom_client();
    *crate::common::DEVICE_NAME.lock().unwrap() = "HarmonyOS".to_owned();
    hbb_common::init_log(false, "rustdesk-ohos");
    if INITIALIZED.set(()).is_err() {
        return -3;
    }
    0
}

#[no_mangle]
pub extern "C" fn rd_ohos_set_callbacks(event: Option<EventCallback>, rgba: Option<RgbaCallback>) {
    *CALLBACKS.write().unwrap() = Callbacks { event, rgba };
}

#[no_mangle]
pub extern "C" fn rd_ohos_version() -> *mut c_char {
    owned(crate::VERSION.to_owned())
}

/// # Safety
/// `value` must be null or an unreleased pointer returned by this module's string functions.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

/// # Safety
/// `peer` and `password` follow the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_connect(
    peer: *const c_char,
    password: *const c_char,
    force_relay: bool,
) -> *mut c_char {
    if INITIALIZED.get().is_none() {
        return std::ptr::null_mut();
    }
    let (Ok(peer), Ok(password)) = (unsafe { argument(peer) }, unsafe { argument(password) })
    else {
        return std::ptr::null_mut();
    };
    if peer.is_empty() || peer.len() > 1024 || peer.chars().any(char::is_control) {
        return std::ptr::null_mut();
    }
    let id = Uuid::new_v4();
    let mut session = Session::<OhosHandler> {
        password: password.to_owned(),
        server_keyboard_enabled: Arc::new(RwLock::new(true)),
        ..Default::default()
    };
    session.ui_handler.id = id;
    session.lc.write().unwrap().initialize(
        peer.to_owned(),
        ConnType::DEFAULT_CONN,
        None,
        force_relay,
        None,
        None,
        None,
        "",
    );
    let session = Arc::new(session);
    SESSIONS.write().unwrap().insert(id, session.clone());
    if std::thread::Builder::new()
        .name("ohos-session".to_owned())
        .spawn(move || {
            io_loop((*session).clone(), 1);
            session.ui_handler.emit("connection_closed", json!({}));
            SESSIONS.write().unwrap().remove(&id);
        })
        .is_err()
    {
        SESSIONS.write().unwrap().remove(&id);
        return std::ptr::null_mut();
    }
    owned(id.to_string())
}

/// # Safety
/// All arguments follow the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_login(
    id: *const c_char,
    password: *const c_char,
    os_user: *const c_char,
    os_password: *const c_char,
    remember: bool,
) -> i32 {
    let (Some(session), Ok(password), Ok(os_user), Ok(os_password)) = (
        unsafe { session(id) },
        unsafe { argument(password) },
        unsafe { argument(os_user) },
        unsafe { argument(os_password) },
    ) else {
        return -1;
    };
    session.login(
        os_user.to_owned(),
        os_password.to_owned(),
        password.to_owned(),
        remember,
    );
    0
}

/// # Safety
/// `id` and `code` follow the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_2fa(id: *const c_char, code: *const c_char, trust: bool) -> i32 {
    let (Some(session), Ok(code)) = (unsafe { session(id) }, unsafe { argument(code) }) else {
        return -1;
    };
    session.send2fa(code.to_owned(), trust);
    0
}

/// # Safety
/// `id` follows the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_close(id: *const c_char) -> i32 {
    let Ok(id) = (unsafe { argument(id) }).and_then(|id| Uuid::parse_str(id).map_err(|_| ()))
    else {
        return -1;
    };
    let session = SESSIONS.write().unwrap().remove(&id);
    if let Some(session) = session {
        session.close();
        return 0;
    }
    -1
}

#[no_mangle]
pub extern "C" fn rd_ohos_close_all() {
    let sessions: Vec<_> = SESSIONS
        .write()
        .unwrap()
        .drain()
        .map(|(_, session)| session)
        .collect();
    for session in sessions {
        session.close();
    }
}

/// # Safety
/// `id` follows the module's C string contract. Coordinates are remote desktop pixels.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_mouse(
    id: *const c_char,
    mask: i32,
    x: i32,
    y: i32,
    modifiers: u32,
) -> i32 {
    let Some(session) = (unsafe { session(id) }) else {
        return -1;
    };
    if !can_control(&session) {
        return -2;
    }
    session.send_mouse(
        mask,
        x,
        y,
        modifiers & 1 != 0,
        modifiers & 2 != 0,
        modifiers & 4 != 0,
        modifiers & 8 != 0,
    );
    0
}

/// # Safety
/// `id` and `key` follow the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_key(id: *const c_char, key: *const c_char, modifiers: u32) -> i32 {
    let (Some(session), Ok(key)) = (unsafe { session(id) }, unsafe { argument(key) }) else {
        return -1;
    };
    if !can_control(&session) {
        return -2;
    }
    if key.len() > 64 || (key.chars().count() != 1 && !crate::client::KEY_MAP.contains_key(key)) {
        return -3;
    }
    session.input_key(
        key,
        false,
        true,
        modifiers & 1 != 0,
        modifiers & 2 != 0,
        modifiers & 4 != 0,
        modifiers & 8 != 0,
    );
    0
}

/// # Safety
/// `id` and `text` follow the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_text(id: *const c_char, text: *const c_char) -> i32 {
    let (Some(session), Ok(text)) = (unsafe { session(id) }, unsafe { argument(text) }) else {
        return -1;
    };
    if !can_control(&session) {
        return -2;
    }
    if text.len() > 65536 {
        return -3;
    }
    session.input_string(text);
    0
}

/// # Safety
/// `id` follows the module's C string contract.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_view_only(id: *const c_char, enabled: bool) -> i32 {
    let Some(session) = (unsafe { session(id) }) else {
        return -1;
    };
    if session.get_toggle_option("view-only".to_owned()) != enabled {
        session.toggle_option("view-only".to_owned());
    }
    0
}

/// # Safety
/// Arguments follow the module's C string contract. Call only on a blocking worker, not a Tokio task.
#[no_mangle]
pub unsafe extern "C" fn rd_ohos_account_request(
    operation: *const c_char,
    payload: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    if INITIALIZED.get().is_none() {
        return std::ptr::null_mut();
    }
    let (Ok(operation), Ok(payload), Ok(token)) = (
        unsafe { argument(operation) },
        unsafe { argument(payload) },
        unsafe { argument(token) },
    ) else {
        return std::ptr::null_mut();
    };
    owned(crate::platform::ohos_account::request(operation, payload, token).to_string())
}
