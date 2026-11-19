#include "frame.h"
#include <multimedia/player_framework/native_avscreen_capture.h>

namespace rustdesk::capture {
// Caller owns the instance and callback lifetime. This probe never starts capture.
OH_AVSCREEN_CAPTURE_ErrCode configure(
    OH_AVScreenCapture* instance, std::int32_t width, std::int32_t height,
    OH_AVScreenCapture_OnStateChange state, OH_AVScreenCapture_OnBufferAvailable data,
    OH_AVScreenCapture_OnError error, void* user_data) {
    if (!instance || !state || !data || !error || !user_data || width <= 0 || height <= 0
        || static_cast<std::size_t>(width) > kMaxFrameBytes / 4
        || static_cast<std::size_t>(height) > kMaxFrameBytes / (static_cast<std::size_t>(width) * 4)) {
        return AV_SCREEN_CAPTURE_ERR_INVALID_VAL;
    }
    OH_AVScreenCaptureConfig config{};
    config.captureMode = OH_CAPTURE_HOME_SCREEN;
    config.dataType = OH_ORIGINAL_STREAM;
    config.videoInfo.videoCapInfo.videoFrameWidth = width;
    config.videoInfo.videoCapInfo.videoFrameHeight = height;
    config.videoInfo.videoCapInfo.videoSource = OH_VIDEO_SOURCE_SURFACE_RGBA;
    auto result = OH_AVScreenCapture_Init(instance, config);
    if (result != AV_SCREEN_CAPTURE_ERR_OK) return result;
    result = OH_AVScreenCapture_SetMicrophoneEnabled(instance, false);
    if (result != AV_SCREEN_CAPTURE_ERR_OK) return result;
    result = OH_AVScreenCapture_SetStateCallback(instance, state, user_data);
    if (result != AV_SCREEN_CAPTURE_ERR_OK) return result;
    result = OH_AVScreenCapture_SetDataCallback(instance, data, user_data);
    if (result != AV_SCREEN_CAPTURE_ERR_OK) return result;
    return OH_AVScreenCapture_SetErrorCallback(instance, error, user_data);
}
}
