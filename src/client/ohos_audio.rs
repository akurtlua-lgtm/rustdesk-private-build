use super::{AudioFormat, AudioFrame};
use crate::audio_resampler::{AudioResampler, AudioResamplerConfig};
use hbb_common::{bail, ResultType};
use magnum_opus::{Channels, Decoder};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use std::{ffi::c_void, ptr, time::Duration};

const DEVICE_RATE: u32 = 48_000;
const LOG_INTERVAL: Duration = Duration::from_secs(10);

#[link(name = "ohaudio")]
extern "C" {
    fn OH_AudioStreamBuilder_Create(builder: *mut *mut c_void, kind: i32) -> i32;
    fn OH_AudioStreamBuilder_Destroy(builder: *mut c_void) -> i32;
    fn OH_AudioStreamBuilder_SetSamplingRate(builder: *mut c_void, rate: i32) -> i32;
    fn OH_AudioStreamBuilder_SetChannelCount(builder: *mut c_void, count: i32) -> i32;
    fn OH_AudioStreamBuilder_SetSampleFormat(builder: *mut c_void, format: i32) -> i32;
    fn OH_AudioStreamBuilder_SetEncodingType(builder: *mut c_void, encoding: i32) -> i32;
    fn OH_AudioStreamBuilder_SetRendererInfo(builder: *mut c_void, usage: i32) -> i32;
    fn OH_AudioStreamBuilder_SetRendererWriteDataCallback(
        builder: *mut c_void,
        callback: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> i32,
        data: *mut c_void,
    ) -> i32;
    fn OH_AudioStreamBuilder_GenerateRenderer(
        builder: *mut c_void,
        renderer: *mut *mut c_void,
    ) -> i32;
    fn OH_AudioRenderer_Start(renderer: *mut c_void) -> i32;
    fn OH_AudioRenderer_Stop(renderer: *mut c_void) -> i32;
    fn OH_AudioRenderer_Release(renderer: *mut c_void) -> i32;
}

struct Builder(*mut c_void);

impl Drop for Builder {
    fn drop(&mut self) {
        if unsafe { OH_AudioStreamBuilder_Destroy(self.0) } != 0 {
            hbb_common::throttled_log!(LOG_INTERVAL, warn, "OHAudio builder release failed");
        }
    }
}

unsafe extern "C" fn write_audio(
    _renderer: *mut c_void,
    context: *mut c_void,
    output: *mut c_void,
    length: i32,
) -> i32 {
    if context.is_null() || output.is_null() || length <= 0 || length % 4 != 0 {
        return -1;
    }
    // OHAudio serializes the renderer's write callbacks. The box stays alive until Release returns.
    let consumer = unsafe { &mut *(context as *mut HeapConsumer<f32>) };
    let output = unsafe { std::slice::from_raw_parts_mut(output as *mut f32, length as usize / 4) };
    let read = consumer.pop_slice(output);
    output[read..].fill(0.0);
    0
}

struct Renderer {
    handle: *mut c_void,
    producer: HeapProducer<f32>,
    consumer: Option<Box<HeapConsumer<f32>>>,
}

// Only the decoder thread controls the renderer; the callback exclusively owns the heap consumer.
unsafe impl Send for Renderer {}

impl Renderer {
    fn new(channels: u16) -> ResultType<Self> {
        let (producer, consumer) =
            HeapRb::new(DEVICE_RATE as usize * channels as usize / 3).split();
        let mut consumer = Box::new(consumer);
        let mut builder = ptr::null_mut();
        if unsafe { OH_AudioStreamBuilder_Create(&mut builder, 1) } != 0 || builder.is_null() {
            bail!("OHAudio renderer builder creation failed");
        }
        let builder = Builder(builder);
        unsafe {
            if OH_AudioStreamBuilder_SetSamplingRate(builder.0, DEVICE_RATE as i32) != 0
                || OH_AudioStreamBuilder_SetChannelCount(builder.0, channels as i32) != 0
                || OH_AudioStreamBuilder_SetSampleFormat(builder.0, 4) != 0
                || OH_AudioStreamBuilder_SetEncodingType(builder.0, 0) != 0
                || OH_AudioStreamBuilder_SetRendererInfo(builder.0, 11) != 0
                || OH_AudioStreamBuilder_SetRendererWriteDataCallback(
                    builder.0,
                    write_audio,
                    (&mut *consumer as *mut HeapConsumer<f32>).cast(),
                ) != 0
            {
                bail!("OHAudio renderer configuration failed");
            }
        }
        let mut handle = ptr::null_mut();
        if unsafe { OH_AudioStreamBuilder_GenerateRenderer(builder.0, &mut handle) } != 0
            || handle.is_null()
        {
            bail!("OHAudio renderer generation failed");
        }
        let renderer = Self {
            handle,
            producer,
            consumer: Some(consumer),
        };
        drop(builder);
        if unsafe { OH_AudioRenderer_Start(renderer.handle) } != 0 {
            bail!("OHAudio renderer startup failed");
        }
        Ok(renderer)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let stop = unsafe { OH_AudioRenderer_Stop(self.handle) };
        if stop != 0 {
            hbb_common::log::trace!("OHAudio stop returned {stop}; releasing renderer");
        }
        if unsafe { OH_AudioRenderer_Release(self.handle) } != 0 {
            hbb_common::throttled_log!(
                LOG_INTERVAL,
                error,
                "OHAudio release failed; retaining callback memory"
            );
            if let Some(context) = self.consumer.take() {
                // A failed Release cannot guarantee that the native callback has stopped.
                std::mem::forget(context);
            }
        }
    }
}

#[derive(Default)]
pub struct AudioHandler {
    decoder: Option<Decoder>,
    decoded: Vec<f32>,
    channels: u16,
    renderer: Option<Renderer>,
    resampler: Option<AudioResampler>,
}

impl AudioHandler {
    fn configure(&mut self, format: AudioFormat) -> ResultType<()> {
        if !matches!(format.channels, 1 | 2)
            || !matches!(format.sample_rate, 8000 | 12000 | 16000 | 24000 | 48000)
        {
            bail!("Unsupported OHOS audio format");
        }
        let channels = if format.channels == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };
        let decoder = Decoder::new(format.sample_rate, channels)?;
        let renderer = Renderer::new(format.channels as u16)?;
        let resampler = if format.sample_rate != DEVICE_RATE {
            Some(AudioResampler::new(AudioResamplerConfig {
                input_rate: format.sample_rate,
                output_rate: DEVICE_RATE,
                channels: format.channels as u16,
            })?)
        } else {
            None
        };
        self.decoded = vec![0.0; format.sample_rate as usize * format.channels as usize];
        self.channels = format.channels as u16;
        self.decoder = Some(decoder);
        self.renderer = Some(renderer);
        self.resampler = resampler;
        Ok(())
    }

    pub fn handle_format(&mut self, format: AudioFormat) {
        if let Err(error) = self.configure(format) {
            *self = Self::default();
            hbb_common::throttled_log!(
                LOG_INTERVAL,
                error,
                "OHAudio initialization failed: {error}"
            );
        }
    }

    pub fn handle_frame(&mut self, frame: AudioFrame) {
        let (Some(decoder), Some(renderer)) = (&mut self.decoder, &mut self.renderer) else {
            return;
        };
        let frames = match decoder.decode_float(&frame.data, &mut self.decoded, false) {
            Ok(frames) => frames,
            Err(error) => {
                hbb_common::throttled_log!(LOG_INTERVAL, warn, "OHOS Opus decode failed: {error}");
                return;
            }
        };
        let channels = self.channels as usize;
        let decoded = &self.decoded[..frames * channels];
        if let Some(resampler) = &mut self.resampler {
            match resampler.process(decoded) {
                Ok(output) => {
                    renderer.producer.push_slice(&output);
                }
                Err(error) => hbb_common::throttled_log!(
                    LOG_INTERVAL,
                    warn,
                    "OHOS audio resampling failed: {error}"
                ),
            }
        } else {
            renderer.producer.push_slice(decoded);
        }
    }
}
