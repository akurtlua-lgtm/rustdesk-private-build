use super::{
    get_service_name, run, CapturerInfo, GenericService, VideoService, VideoSource,
    FRAME_FETCHED_NOTIFIERS, INVALID_PRIVACY_MODE_CONN_ID,
};
use hbb_common::{
    tokio::sync::{mpsc::unbounded_channel, Mutex as TokioMutex},
    ResultType,
};
use scrap::{mobile_frame::FrameInbox, Capturer, Display};
use std::sync::Arc;

pub(in crate::server) fn new_for_source(source: Arc<FrameInbox>) -> ResultType<GenericService> {
    Display::from_source(source.clone())?;
    FRAME_FETCHED_NOTIFIERS
        .lock()
        .unwrap()
        .entry(0)
        .or_insert_with(|| {
            let (tx, rx) = unbounded_channel();
            (tx, Arc::new(TokioMutex::new(rx)))
        });
    let service = VideoService {
        sp: GenericService::new(get_service_name(VideoSource::Monitor, 0), true),
        idx: 0,
        source: VideoSource::Monitor,
        capture_source: source,
    };
    GenericService::run(&service, run);
    Ok(service.sp)
}

pub(super) fn get_capturer(source: Arc<FrameInbox>) -> ResultType<CapturerInfo> {
    let display = Display::from_source(source)?;
    let width = display.width();
    let height = display.height();
    Ok(CapturerInfo {
        origin: (0, 0),
        width,
        height,
        ndisplay: 1,
        current: 0,
        privacy_mode_id: INVALID_PRIVACY_MODE_CONN_ID,
        _capturer_privacy_mode_id: INVALID_PRIVACY_MODE_CONN_ID,
        capturer: Box::new(Capturer::new(display)?),
    })
}
