#pragma once
#include "delivery.h"
#include <cstdint>
#include <memory>
#include <multimedia/player_framework/native_avscreen_capture.h>

namespace rustdesk::capture {
class Capture {
public:
    // The application serializes create/start/stop; SDK callbacks use the separate Delivery gate.
    static std::unique_ptr<Capture> create(std::int32_t width, std::int32_t height,
                                           std::shared_ptr<Delivery> delivery, int& error);
    ~Capture();
    Capture(const Capture&) = delete;
    Capture& operator=(const Capture&) = delete;
    int start();
    int stop();
private:
    Capture(OH_AVScreenCapture* instance, std::uintptr_t token,
             std::shared_ptr<Delivery> delivery);
    OH_AVScreenCapture* instance_;
    std::uintptr_t token_;
    std::shared_ptr<Delivery> delivery_;
    bool start_requested_ = false;
};
}
