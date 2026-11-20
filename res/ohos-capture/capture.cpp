#include "capture.h"
#include <limits>
#include <map>
#include <new>
#include <cstdio>
#include <utility>
#include <multimedia/player_framework/native_avbuffer.h>
#include <native_buffer/native_buffer.h>

namespace rustdesk::capture {
OH_AVSCREEN_CAPTURE_ErrCode configure(OH_AVScreenCapture*, std::int32_t, std::int32_t,
    OH_AVScreenCapture_OnStateChange, OH_AVScreenCapture_OnBufferAvailable,
    OH_AVScreenCapture_OnError, void*);

namespace {
std::mutex registry_mutex;
std::map<std::uintptr_t, std::shared_ptr<Delivery>> registry;
std::uintptr_t next_token = 1;

std::shared_ptr<Delivery> delivery_for(void* token) {
    std::lock_guard<std::mutex> lock(registry_mutex);
    auto found = registry.find(reinterpret_cast<std::uintptr_t>(token));
    return found == registry.end() ? nullptr : found->second;
}
void on_state(OH_AVScreenCapture*, OH_AVScreenCaptureStateCode state, void* token) {
    auto delivery = delivery_for(token);
    if (!delivery) return;
    if (state == OH_SCREEN_CAPTURE_STATE_STARTED) {
        delivery->authorize();
    } else if (state != OH_SCREEN_CAPTURE_STATE_MIC_UNAVAILABLE
               && state != OH_SCREEN_CAPTURE_STATE_MIC_MUTED_BY_USER
               && state != OH_SCREEN_CAPTURE_STATE_MIC_UNMUTED_BY_USER
               && state != OH_SCREEN_CAPTURE_STATE_EXIT_PRIVATE_SCENE) {
        delivery->close(static_cast<int>(state));
    }
}
void on_error(OH_AVScreenCapture*, std::int32_t error, void* token) {
    if (auto delivery = delivery_for(token)) delivery->close(error);
}
struct BufferLease {
    OH_NativeBuffer* buffer;
    std::shared_ptr<Delivery> delivery;
    bool mapped = false;
    ~BufferLease() {
        if (mapped && OH_NativeBuffer_Unmap(buffer) != 0) delivery->close(-1);
        if (OH_NativeBuffer_Unreference(buffer) != 0) delivery->close(-1);
    }
};
void on_buffer(OH_AVScreenCapture*, OH_AVBuffer* buffer,
               OH_AVScreenCaptureBufferType type, std::int64_t, void* token) {
    auto delivery = delivery_for(token);
    if (!delivery || delivery->is_closed() || type != OH_SCREEN_CAPTURE_BUFFERTYPE_VIDEO) return;
    auto* native = OH_AVBuffer_GetNativeBuffer(buffer);
    if (!native) { delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL); return; }
    BufferLease lease{native, delivery};
    OH_NativeBuffer_Config config{};
    OH_NativeBuffer_GetConfig(native, &config);
    if ((config.usage & NATIVEBUFFER_USAGE_CPU_READ) == 0) {
        delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL); return;
    }
    PixelFormat format;
    if (config.format == NATIVEBUFFER_PIXEL_FMT_BGRA_8888) format = PixelFormat::bgra;
    else if (config.format == NATIVEBUFFER_PIXEL_FMT_RGBA_8888) format = PixelFormat::rgba;
    else { delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL); return; }
    void* address = nullptr;
    OH_NativeBuffer_Planes planes{};
    if (OH_NativeBuffer_MapPlanes(native, &address, &planes) != 0) {
        delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL); return;
    }
    lease.mapped = true;
    if (!address || config.width <= 0 || config.height <= 0 || planes.planeCount != 1
        || planes.planes[0].offset != 0 || planes.planes[0].columnStride != 4
        || planes.planes[0].rowStride == 0
        || static_cast<std::size_t>(config.height) > kMaxFrameBytes / planes.planes[0].rowStride) {
        delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL); return;
    }
    const auto stride = planes.planes[0].rowStride;
    // MapPlanes describes an OS-owned packed allocation; no AVBuffer memory-capacity assumption.
    auto result = copy_frame(static_cast<const std::uint8_t*>(address),
        static_cast<std::size_t>(stride) * config.height,
        config.width, config.height, stride, format);
    if (result.error != FrameError::none) {
        delivery->close(AV_SCREEN_CAPTURE_ERR_INVALID_VAL);
    } else {
        delivery->publish(std::move(result.frame));
    }
}
}

Capture::Capture(OH_AVScreenCapture* instance, std::uintptr_t token,
                 std::shared_ptr<Delivery> delivery)
    : instance_(instance), token_(token), delivery_(std::move(delivery)) {}

std::unique_ptr<Capture> Capture::create(std::int32_t width, std::int32_t height,
                                         std::shared_ptr<Delivery> delivery, int& error) {
    error = AV_SCREEN_CAPTURE_ERR_INVALID_VAL;
    if (!delivery || width <= 0 || height <= 0) return nullptr;
    std::uintptr_t token;
    {
        std::lock_guard<std::mutex> lock(registry_mutex);
        if (!registry.empty() || next_token == std::numeric_limits<std::uintptr_t>::max()) return nullptr;
        token = next_token++;
        registry.emplace(token, delivery);
    }
    auto* instance = OH_AVScreenCapture_Create();
    if (!instance) {
        std::lock_guard<std::mutex> lock(registry_mutex); registry.erase(token); return nullptr;
    }
    auto result = std::unique_ptr<Capture>(new (std::nothrow) Capture(instance, token, delivery));
    if (!result) {
        error = AV_SCREEN_CAPTURE_ERR_NO_MEMORY;
        if (OH_AVScreenCapture_Release(instance) != AV_SCREEN_CAPTURE_ERR_OK) {
            delivery->close(error);
            std::fputs("Failed to release uninitialized capture instance\n", stderr);
        } else {
            std::lock_guard<std::mutex> lock(registry_mutex); registry.erase(token);
        }
        return nullptr;
    }
    error = configure(instance, width, height, on_state, on_buffer, on_error, reinterpret_cast<void*>(token));
    if (error != AV_SCREEN_CAPTURE_ERR_OK) return nullptr;
    return result;
}
Capture::~Capture() {
    if (stop() != AV_SCREEN_CAPTURE_ERR_OK) {
        std::fputs("Capture cleanup failed; system capture stop must be checked\n", stderr);
    }
}
int Capture::start() {
    if (!instance_ || start_requested_ || delivery_->is_closed()) return AV_SCREEN_CAPTURE_ERR_INVALID_STATE;
    const auto result = OH_AVScreenCapture_StartScreenCapture(instance_);
    start_requested_ = result == AV_SCREEN_CAPTURE_ERR_OK;
    if (result != AV_SCREEN_CAPTURE_ERR_OK) delivery_->close(result);
    return result;
}
int Capture::stop() {
    delivery_->close(OH_SCREEN_CAPTURE_STATE_STOPPED_BY_CALL);
    if (!instance_) return AV_SCREEN_CAPTURE_ERR_OK;
    int stop_result = AV_SCREEN_CAPTURE_ERR_OK;
    if (start_requested_) {
        stop_result = OH_AVScreenCapture_StopScreenCapture(instance_);
        if (stop_result == AV_SCREEN_CAPTURE_ERR_OK) start_requested_ = false;
    }
    const auto result = OH_AVScreenCapture_Release(instance_);
    if (result == AV_SCREEN_CAPTURE_ERR_OK) {
        instance_ = nullptr;
        start_requested_ = false;
        std::lock_guard<std::mutex> lock(registry_mutex); registry.erase(token_);
    }
    return stop_result != AV_SCREEN_CAPTURE_ERR_OK ? stop_result : result;
}
}
