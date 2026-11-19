use crate::message_proto::{message, misc, option_message::BoolOption, Message, OptionMessage};

const UNSUPPORTED: &str = "Unsupported operation in a read-only mobile session";

fn restrict_options(options: &mut OptionMessage) {
    *options = OptionMessage {
        image_quality: options.image_quality,
        custom_image_quality: options.custom_image_quality,
        custom_fps: options.custom_fps,
        supported_decoding: std::mem::take(&mut options.supported_decoding),
        disable_audio: BoolOption::Yes.into(),
        disable_clipboard: BoolOption::Yes.into(),
        disable_keyboard: BoolOption::Yes.into(),
        disable_camera: BoolOption::Yes.into(),
        enable_file_transfer: BoolOption::No.into(),
        block_input: BoolOption::No.into(),
        privacy_mode: BoolOption::No.into(),
        lock_after_session_end: BoolOption::No.into(),
        terminal_persistent: BoolOption::No.into(),
        ..Default::default()
    };
}

pub fn restrict_view_only_message(mut message: Message) -> Result<Message, &'static str> {
    match message.union.as_mut() {
        Some(message::Union::LoginRequest(login)) => {
            if login.union.is_some()
                || login.os_login.as_ref().is_some_and(|os_login| {
                    !os_login.username.is_empty() || !os_login.password.is_empty()
                })
            {
                return Err(UNSUPPORTED);
            }
            if let Some(options) = login.option.as_mut() {
                restrict_options(options);
            }
        }
        Some(message::Union::PublicKey(_))
        | Some(message::Union::TestDelay(_))
        | Some(message::Union::Auth2fa(_)) => {}
        Some(message::Union::Misc(misc)) => match misc.union.as_mut() {
            Some(misc::Union::Option(options)) => restrict_options(options),
            Some(misc::Union::CloseReason(_))
            | Some(misc::Union::RefreshVideo(_))
            | Some(misc::Union::VideoReceived(_))
            | Some(misc::Union::AutoAdjustFps(_))
            | Some(misc::Union::FullSpeedFps(_))
            | Some(misc::Union::SupportedEncoding(_)) => {}
            Some(misc::Union::ClientRecordStatus(false)) => {}
            Some(misc::Union::SwitchDisplay(display)) if display.display == 0 => {}
            Some(misc::Union::RefreshVideoDisplay(0)) => {}
            Some(misc::Union::MessageQuery(query)) if query.switch_display == 0 => {}
            Some(misc::Union::CaptureDisplays(displays))
                if displays
                    .add
                    .iter()
                    .chain(&displays.sub)
                    .chain(&displays.set)
                    .all(|index| *index == 0) => {}
            _ => return Err(UNSUPPORTED),
        },
        _ => return Err(UNSUPPORTED),
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_proto::{
        login_request, Clipboard, Cliprdr, FileAction, FileTransfer, KeyEvent, LoginRequest, Misc,
        MouseEvent, MultiClipboards, OSLogin, OptionMessage, PointerDeviceEvent, PortForward,
        PortForwardChannel, SwitchSidesResponse, Terminal, TerminalAction, TestDelay, ViewCamera,
    };

    #[test]
    fn preserves_device_authentication_and_video_options_without_control_permissions() {
        let mut message = Message::new();
        message.set_login_request(LoginRequest {
            username: "synthetic-device".into(),
            password: bytes::Bytes::from_static(b"synthetic-authentication"),
            option: Some(OptionMessage {
                custom_fps: 30,
                custom_image_quality: 42,
                disable_keyboard: BoolOption::No.into(),
                enable_file_transfer: BoolOption::Yes.into(),
                block_input: BoolOption::Yes.into(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        });
        let message = restrict_view_only_message(message).unwrap();
        let Some(message::Union::LoginRequest(login)) = message.union else {
            panic!("login");
        };
        assert_eq!(login.username, "synthetic-device");
        assert_eq!(login.password.as_ref(), b"synthetic-authentication");
        let options = login.option.unwrap();
        assert_eq!((options.custom_fps, options.custom_image_quality), (30, 42));
        assert_eq!(options.disable_keyboard.enum_value(), Ok(BoolOption::Yes));
        assert_eq!(options.disable_clipboard.enum_value(), Ok(BoolOption::Yes));
        assert_eq!(
            options.enable_file_transfer.enum_value(),
            Ok(BoolOption::No)
        );
        assert_eq!(options.block_input.enum_value(), Ok(BoolOption::No));
    }

    #[test]
    fn refuses_input_and_device_control_but_retains_video_flow_feedback() {
        for union in [
            message::Union::MouseEvent(MouseEvent::new()),
            message::Union::KeyEvent(KeyEvent::new()),
            message::Union::PointerDeviceEvent(PointerDeviceEvent::new()),
            message::Union::OsLogin(OSLogin::new()),
            message::Union::Clipboard(Clipboard::new()),
            message::Union::MultiClipboards(MultiClipboards::new()),
            message::Union::Cliprdr(Cliprdr::new()),
            message::Union::FileAction(FileAction::new()),
            message::Union::TerminalAction(TerminalAction::new()),
            message::Union::PortForwardChannel(PortForwardChannel::new()),
            message::Union::SwitchSidesResponse(SwitchSidesResponse::new()),
        ] {
            assert!(restrict_view_only_message(Message {
                union: Some(union),
                ..Default::default()
            })
            .is_err());
        }
        let mut control = Message::new();
        let mut misc = Misc::new();
        misc.set_restart_remote_device(true);
        control.set_misc(misc);
        assert!(restrict_view_only_message(control).is_err());
        let mut delay = Message::new();
        delay.set_test_delay(TestDelay::new());
        assert!(restrict_view_only_message(delay).is_ok());
        let mut feedback = Message::new();
        let mut misc = Misc::new();
        misc.set_video_received(true);
        feedback.set_misc(misc);
        assert!(restrict_view_only_message(feedback).is_ok());
    }

    #[test]
    fn refuses_non_desktop_sessions_and_operating_system_login() {
        for union in [
            login_request::Union::FileTransfer(FileTransfer::new()),
            login_request::Union::PortForward(PortForward::new()),
            login_request::Union::Terminal(Terminal::new()),
            login_request::Union::ViewCamera(ViewCamera::new()),
        ] {
            let mut message = Message::new();
            message.set_login_request(LoginRequest {
                union: Some(union),
                ..Default::default()
            });
            assert!(restrict_view_only_message(message).is_err());
        }
        let mut message = Message::new();
        message.set_login_request(LoginRequest {
            os_login: Some(OSLogin {
                username: "synthetic-os-user".into(),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        });
        assert!(restrict_view_only_message(message).is_err());
    }
}
