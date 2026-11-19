use super::{display_service, Server, ServerPtr, ServerPtrWeak};
use base::mobile_host_stop::{StopReceiver, StopSignal};
use hbb_common::{bail, ResultType};
use scrap::{mobile_frame::FrameInbox, CaptureLease};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock, Weak},
};

static CURRENT: Mutex<Weak<Scope>> = Mutex::new(Weak::new());

struct Scope {
    server: ServerPtrWeak,
    source: Arc<FrameInbox>,
    stop: StopSignal,
}

pub(crate) struct ScopedHost {
    _capture: CaptureLease,
    scope: Arc<Scope>,
    server: ServerPtr,
}

pub(crate) fn prepare(source: Arc<FrameInbox>) -> ResultType<ScopedHost> {
    crate::common::init_sodium();
    let mut current = CURRENT.lock().unwrap();
    if current.upgrade().is_some() {
        bail!("A capture session is already prepared");
    }
    let capture = scrap::begin_capture(source.clone())?;
    let mut server = Server {
        connections: HashMap::new(),
        services: HashMap::new(),
        id_count: hbb_common::rand::random::<i32>() % 1000 + 1000,
    };
    server.add_service(Box::new(display_service::new()));
    let server = Arc::new(RwLock::new(server));
    let scope = Arc::new(Scope {
        server: Arc::downgrade(&server),
        source,
        stop: StopSignal::new(),
    });
    *current = Arc::downgrade(&scope);
    Ok(ScopedHost {
        _capture: capture,
        scope,
        server,
    })
}

impl ScopedHost {
    pub(crate) fn server(&self) -> ServerPtr {
        self.server.clone()
    }
}

impl Drop for ScopedHost {
    fn drop(&mut self) {
        self.scope.stop.stop();
        self.scope.source.close();
        let mut current = CURRENT.lock().unwrap();
        if current.ptr_eq(&Arc::downgrade(&self.scope)) {
            *current = Weak::new();
        }
    }
}

pub(crate) fn stop_receiver(server: &ServerPtrWeak, id: i32) -> Option<StopReceiver> {
    let scope = CURRENT.lock().unwrap().upgrade()?;
    if !scope.server.ptr_eq(server)
        || scope.stop.is_stopped()
        || scope.source.dimensions().is_none()
    {
        return None;
    }
    Some(scope.stop.receiver(id))
}

pub(crate) fn source_for(server: &ServerPtrWeak) -> Option<Arc<FrameInbox>> {
    let scope = CURRENT.lock().unwrap().upgrade()?;
    if !scope.server.ptr_eq(server)
        || scope.stop.is_stopped()
        || scope.source.dimensions().is_none()
    {
        return None;
    }
    Some(scope.source.clone())
}
