#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>

namespace rustdesk::capture {
constexpr std::size_t kMaxFrameBytes = 32 * 1024 * 1024;
enum class PixelFormat { bgra, rgba };
enum class FrameError { none, invalid_layout, unsupported_format, allocation_failed };
struct Frame {
    std::size_t width = 0;
    std::size_t height = 0;
    std::size_t stride = 0;
    std::unique_ptr<std::uint8_t[]> pixels;
};
struct FrameResult {
    FrameError error;
    Frame frame;
};
FrameResult copy_frame(const std::uint8_t* data, std::size_t capacity,
                       std::int32_t width, std::int32_t height,
                       std::size_t stride, PixelFormat format);
}
