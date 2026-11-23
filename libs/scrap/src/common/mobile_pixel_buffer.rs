use super::{mobile_frame::BgraFrame, Pixfmt, TraitPixelBuffer};

pub struct PixelBuffer<'a> {
    frame: &'a BgraFrame,
}

impl<'a> PixelBuffer<'a> {
    pub fn new(frame: &'a BgraFrame) -> Self {
        Self { frame }
    }
}

impl TraitPixelBuffer for PixelBuffer<'_> {
    fn data(&self) -> &[u8] {
        self.frame.data()
    }

    fn width(&self) -> usize {
        self.frame.width()
    }

    fn height(&self) -> usize {
        self.frame.height()
    }

    fn stride(&self) -> Vec<usize> {
        vec![self.frame.stride()]
    }

    fn pixfmt(&self) -> Pixfmt {
        Pixfmt::BGRA
    }
}
