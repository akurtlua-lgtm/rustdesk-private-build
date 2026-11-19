use super::{
    mobile_frame::{BgraFrame, FrameInbox},
    Frame, PixelBuffer, TraitCapturer,
};
use std::{
    io,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

static SOURCE: Mutex<Option<Weak<FrameInbox>>> = Mutex::new(None);

pub struct CaptureLease {
    source: Arc<FrameInbox>,
}

pub fn begin_capture(source: Arc<FrameInbox>) -> io::Result<CaptureLease> {
    let mut registered = SOURCE.lock().unwrap();
    if registered.as_ref().and_then(Weak::upgrade).is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Capture source is already registered",
        ));
    }
    if source.dimensions().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Capture has no valid frame",
        ));
    }
    *registered = Some(Arc::downgrade(&source));
    Ok(CaptureLease { source })
}

impl Drop for CaptureLease {
    fn drop(&mut self) {
        self.source.close();
        let mut registered = SOURCE.lock().unwrap();
        if registered
            .as_ref()
            .is_some_and(|source| source.ptr_eq(&Arc::downgrade(&self.source)))
        {
            *registered = None;
        }
    }
}

#[derive(Clone)]
pub struct Display {
    source: Arc<FrameInbox>,
    dimensions: (usize, usize),
}

impl Display {
    pub fn from_source(source: Arc<FrameInbox>) -> io::Result<Self> {
        let dimensions = source.dimensions().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Capture source is unavailable")
        })?;
        Ok(Self { source, dimensions })
    }

    pub fn all() -> io::Result<Vec<Self>> {
        let source = SOURCE.lock().unwrap().as_ref().and_then(Weak::upgrade);
        let Some(source) = source else {
            return Ok(Vec::new());
        };
        let Some(dimensions) = source.dimensions() else {
            return Ok(Vec::new());
        };
        Ok(vec![Self { source, dimensions }])
    }

    pub fn primary() -> io::Result<Self> {
        Self::all()?
            .into_iter()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No active capture display"))
    }

    pub fn width(&self) -> usize {
        self.dimensions.0
    }
    pub fn height(&self) -> usize {
        self.dimensions.1
    }
    pub fn origin(&self) -> (i32, i32) {
        (0, 0)
    }
    pub fn is_online(&self) -> bool {
        self.source.dimensions().is_some()
    }
    pub fn is_primary(&self) -> bool {
        true
    }
    pub fn name(&self) -> String {
        "iOS screen".into()
    }
}

pub struct Capturer {
    display: Display,
    current: Option<BgraFrame>,
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Self> {
        if display.source.dimensions() != Some(display.dimensions) {
            return Err(io::Error::other(
                "Capture dimensions changed or capture stopped",
            ));
        }
        Ok(Self {
            display,
            current: None,
        })
    }

    pub fn width(&self) -> usize {
        self.display.width()
    }
    pub fn height(&self) -> usize {
        self.display.height()
    }
}

impl TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, _timeout: Duration) -> io::Result<Frame<'a>> {
        self.current = Some(self.display.source.take_frame(self.display.dimensions)?);
        let frame = self
            .current
            .as_ref()
            .ok_or_else(|| io::Error::other("Capture frame unavailable"))?;
        Ok(Frame::PixelBuffer(PixelBuffer::new(frame)))
    }
}
