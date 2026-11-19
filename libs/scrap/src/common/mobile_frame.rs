use std::{io, sync::Mutex};

pub const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug)]
pub struct BgraFrame {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl BgraFrame {
    fn copy_from(data: &[u8], width: usize, height: usize, stride: usize) -> io::Result<Self> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "Invalid BGRA frame layout");
        if width == 0 || height == 0 {
            return Err(invalid());
        }
        let row_bytes = width.checked_mul(4).ok_or_else(invalid)?;
        let size = row_bytes.checked_mul(height).ok_or_else(invalid)?;
        let required = (height - 1)
            .checked_mul(stride)
            .and_then(|offset| offset.checked_add(row_bytes))
            .ok_or_else(invalid)?;
        if stride < row_bytes || required > data.len() || size > MAX_FRAME_BYTES {
            return Err(invalid());
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(size)
            .map_err(|_| io::Error::other("Unable to allocate bounded capture frame"))?;
        for row in 0..height {
            let offset = row * stride;
            pixels.extend_from_slice(&data[offset..offset + row_bytes]);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn stride(&self) -> usize {
        self.width * 4
    }

    pub fn data(&self) -> &[u8] {
        &self.pixels
    }
}

#[derive(Default)]
struct InboxState {
    closed: bool,
    dimensions: Option<(usize, usize)>,
    latest: Option<BgraFrame>,
}

#[derive(Default)]
pub struct FrameInbox {
    state: Mutex<InboxState>,
}

impl FrameInbox {
    pub fn publish_bgra(
        &self,
        data: &[u8],
        width: usize,
        height: usize,
        stride: usize,
    ) -> io::Result<()> {
        if self.state.lock().unwrap().closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Capture is closed",
            ));
        }
        let frame = BgraFrame::copy_from(data, width, height, stride)?;
        let previous = {
            let mut state = self.state.lock().unwrap();
            if state.closed {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Capture is closed",
                ));
            }
            state.dimensions = Some((width, height));
            state.latest.replace(frame)
        };
        drop(previous);
        Ok(())
    }

    pub fn dimensions(&self) -> Option<(usize, usize)> {
        self.state.lock().unwrap().dimensions
    }

    pub fn take_frame(&self, expected: (usize, usize)) -> io::Result<BgraFrame> {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Capture is closed",
            ));
        }
        if state
            .latest
            .as_ref()
            .is_some_and(|frame| (frame.width, frame.height) != expected)
        {
            // Leave the new frame for the capturer rebuilt by the display service.
            return Err(io::Error::other("Capture dimensions changed"));
        }
        state
            .latest
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "No new capture frame"))
    }

    pub fn close(&self) {
        let previous = {
            let mut state = self.state.lock().unwrap();
            state.closed = true;
            state.dimensions = None;
            state.latest.take()
        };
        drop(previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_padded_rows_and_rejects_invalid_layouts() {
        let frame = BgraFrame::copy_from(&[1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8], 1, 2, 8).unwrap();
        assert_eq!((frame.width(), frame.height(), frame.stride()), (1, 2, 4));
        assert_eq!(frame.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        for (width, height, stride) in [(0, 1, 4), (1, 2, 3), (2, 2, 8), (usize::MAX, 2, 8)] {
            assert!(BgraFrame::copy_from(&[0; 12], width, height, stride).is_err());
        }
    }

    #[test]
    fn returns_only_latest_frame_to_matching_encoder_dimensions() {
        let inbox = FrameInbox::default();
        inbox.publish_bgra(&[1; 8], 2, 1, 8).unwrap();
        inbox.publish_bgra(&[2; 4], 1, 1, 4).unwrap();
        assert_eq!(inbox.dimensions(), Some((1, 1)));
        assert_eq!(
            inbox.take_frame((2, 1)).unwrap_err().kind(),
            io::ErrorKind::Other
        );
        assert_eq!(inbox.take_frame((1, 1)).unwrap().data(), &[2; 4]);
        assert_eq!(inbox.dimensions(), Some((1, 1)));
        assert_eq!(
            inbox.take_frame((1, 1)).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[test]
    fn closed_inbox_never_delivers_or_accepts_another_frame() {
        let inbox = FrameInbox::default();
        inbox.publish_bgra(&[1; 4], 1, 1, 4).unwrap();
        inbox.close();
        assert_eq!(inbox.dimensions(), None);
        assert_eq!(
            inbox.take_frame((1, 1)).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_eq!(
            inbox.publish_bgra(&[2; 4], 1, 1, 4).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}
