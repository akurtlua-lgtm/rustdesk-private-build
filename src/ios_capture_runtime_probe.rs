#![cfg(all(target_os = "ios", feature = "mobile-view-only-host"))]

#[cfg(test)]
mod tests {
    use base::message_proto::video_frame;
    use hbb_common::sodiumoxide::{self, crypto::box_};
    use scrap::{
        codec::{EncoderApi, EncoderCfg},
        mobile_frame::FrameInbox,
        vpxcodec::{VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig, VpxVideoCodecId},
        Frame, GoogleImage, ImageFormat, ImageRgb, PixelBuffer,
    };

    #[test]
    fn synthetic_bgra_vp8_authenticated_roundtrip() {
        sodiumoxide::init().unwrap();
        let (width, height) = (128, 96);
        let mut pixels = Vec::with_capacity(width * height * 4);
        for _ in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(if x < width / 2 {
                    &[0, 0, 255, 255]
                } else {
                    &[255, 0, 0, 255]
                });
            }
        }
        let inbox = FrameInbox::default();
        inbox
            .publish_bgra(&pixels, width, height, width * 4)
            .unwrap();
        let owned = inbox.take_frame((width, height)).unwrap();
        let frame = Frame::PixelBuffer(PixelBuffer::new(&owned));
        let mut encoder = VpxEncoder::new(
            EncoderCfg::VPX(VpxEncoderConfig {
                width: width as _,
                height: height as _,
                quality: 2.0,
                codec: VpxVideoCodecId::VP8,
                keyframe_interval: None,
            }),
            false,
        )
        .unwrap();
        let (mut yuv, mut temporary) = (Vec::new(), Vec::new());
        let input = frame
            .to(encoder.yuvfmt(), &mut yuv, &mut temporary)
            .unwrap();
        let message = encoder.encode_to_message(input, 0).unwrap();
        let Some(video_frame::Union::Vp8s(encoded)) = message.union else {
            panic!("Expected actual VP8 production packets");
        };
        assert!(!encoded.frames.is_empty());
        let (sender_public, sender_secret) = box_::gen_keypair();
        let (receiver_public, receiver_secret) = box_::gen_keypair();
        let mut decoder = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP8,
        })
        .unwrap();
        let mut rgb = ImageRgb::new(ImageFormat::ARGB, 1);
        for packet in encoded.frames {
            let nonce = box_::gen_nonce();
            let cipher = box_::seal(&packet.data, &nonce, &receiver_public, &sender_secret);
            let plain = box_::open(&cipher, &nonce, &sender_public, &receiver_secret).unwrap();
            assert_eq!(plain.as_slice(), packet.data.as_ref());
            for image in decoder.decode(&plain).unwrap() {
                image.to(&mut rgb);
            }
        }
        for image in decoder.flush().unwrap() {
            image.to(&mut rgb);
        }
        assert_eq!(
            (rgb.w, rgb.h, rgb.raw.len()),
            (width, height, width * height * 4)
        );
        let left = (height / 2 * width + width / 4) * 4;
        let right = (height / 2 * width + width * 3 / 4) * 4;
        assert!(rgb.raw[left + 2] > 200 && rgb.raw[left] < 50);
        assert!(rgb.raw[right] > 200 && rgb.raw[right + 2] < 50);
        inbox.close();
    }
}
