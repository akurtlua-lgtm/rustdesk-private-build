pub const NAME: &str = "audio";

pub fn get_voice_call_input_device() -> Option<String> {
    None
}

pub fn set_voice_call_input_device(_device: Option<String>, _set_if_present: bool) {
    restart();
}

pub fn restart() {
    hbb_common::log::trace!("Audio capture is unsupported in read-only mobile sharing");
}
