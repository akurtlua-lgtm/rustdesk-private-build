#include "frame.h"
#include <cassert>
#include <iostream>
#include <limits>

using namespace rustdesk::capture;
int main() {
    const std::uint8_t bgra[] = {1, 2, 3, 255, 99, 99, 99, 99, 4, 5, 6, 255};
    auto padded = copy_frame(bgra, sizeof(bgra), 1, 2, 8, PixelFormat::bgra);
    assert(padded.error == FrameError::none && padded.frame.width == 1 && padded.frame.height == 2);
    assert(padded.frame.stride == 4 && padded.frame.pixels[0] == 1 && padded.frame.pixels[4] == 4);
    const std::uint8_t rgba[] = {33, 22, 11, 255};
    auto converted = copy_frame(rgba, sizeof(rgba), 1, 1, 4, PixelFormat::rgba);
    assert(converted.error == FrameError::none);
    assert(converted.frame.pixels[0] == 11 && converted.frame.pixels[1] == 22);
    assert(converted.frame.pixels[2] == 33 && converted.frame.pixels[3] == 255);
    assert(copy_frame(bgra, sizeof(bgra), 1, 2, std::numeric_limits<std::size_t>::max(), PixelFormat::bgra).error == FrameError::invalid_layout);
    assert(copy_frame(bgra, sizeof(bgra), 1, 2, 3, PixelFormat::bgra).error == FrameError::invalid_layout);
    assert(copy_frame(bgra, sizeof(bgra) - 1, 1, 2, 8, PixelFormat::bgra).error == FrameError::invalid_layout);
    assert(copy_frame(bgra, sizeof(bgra), 8192, 8192, 32768, PixelFormat::bgra).error == FrameError::invalid_layout);
    std::cout << "3 owned-frame conversion tests passed\n";
}
