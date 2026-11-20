#include "frame.h"
#include <cstring>
#include <new>
#include <utility>

namespace rustdesk::capture {
FrameResult copy_frame(const std::uint8_t* data, std::size_t capacity,
                       std::int32_t width, std::int32_t height,
                       std::size_t stride, PixelFormat format) {
    if (format != PixelFormat::bgra && format != PixelFormat::rgba) {
        return {FrameError::unsupported_format, {}};
    }
    if (!data || width <= 0 || height <= 0
        || static_cast<std::size_t>(width) > kMaxFrameBytes / 4) {
        return {FrameError::invalid_layout, {}};
    }
    const auto row = static_cast<std::size_t>(width) * 4;
    const auto rows = static_cast<std::size_t>(height);
    if (rows > kMaxFrameBytes / row || stride < row || capacity < row
        || (rows > 1 && stride > (capacity - row) / (rows - 1))) {
        return {FrameError::invalid_layout, {}};
    }
    auto pixels = std::unique_ptr<std::uint8_t[]>(new (std::nothrow) std::uint8_t[row * rows]);
    if (!pixels) {
        return {FrameError::allocation_failed, {}};
    }
    for (std::size_t y = 0; y < rows; ++y) {
        const auto* source = data + y * stride;
        auto* destination = pixels.get() + y * row;
        if (format == PixelFormat::bgra) {
            std::memcpy(destination, source, row);
        } else {
            for (std::size_t x = 0; x < row; x += 4) {
                destination[x] = source[x + 2];
                destination[x + 1] = source[x + 1];
                destination[x + 2] = source[x];
                destination[x + 3] = source[x + 3];
            }
        }
    }
    return {FrameError::none, {static_cast<std::size_t>(width), rows, row, std::move(pixels)}};
}
}
