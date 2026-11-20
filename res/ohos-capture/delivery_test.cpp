#include "delivery.h"
#include <cassert>
#include <condition_variable>
#include <future>
#include <iostream>
#include <thread>

using namespace rustdesk::capture;
Frame frame() {
    const std::uint8_t pixels[] = {11, 22, 33, 255};
    return std::move(copy_frame(pixels, sizeof(pixels), 1, 1, 4, PixelFormat::bgra).frame);
}
int main() {
    int frames = 0;
    int stopped = 0;
    Delivery first([&](Frame value) { assert(value.pixels[0] == 11); ++frames; return true; },
                   [&](int reason) { assert(reason == 2); ++stopped; });
    first.publish(frame()); assert(frames == 0);
    first.authorize(); first.publish(frame()); assert(frames == 1);
    first.close(2); first.authorize(); first.publish(frame()); first.close(9);
    assert(frames == 1 && stopped == 1 && first.is_closed());

    std::promise<void> entered;
    std::promise<void> release;
    auto released = release.get_future();
    bool delivered = false;
    bool closed = false;
    Delivery parallel([&](Frame value) {
        entered.set_value(); released.wait(); assert(value.pixels[2] == 33); delivered = true; return true;
    }, [&](int) { assert(delivered); closed = true; });
    parallel.authorize();
    std::thread producer([&] { parallel.publish(frame()); });
    entered.get_future().wait();
    std::thread closer([&] { parallel.close(4); });
    release.set_value(); producer.join(); closer.join();
    parallel.publish(frame()); assert(delivered && closed && parallel.is_closed());

    int failed = 0;
    Delivery rejected([](Frame) { return false; }, [&](int reason) { assert(reason == -1); ++failed; });
    rejected.authorize(); rejected.publish(frame()); rejected.publish(frame()); rejected.authorize();
    assert(rejected.is_closed() && failed == 1);
    std::cout << "3 authorized delivery and stop tests passed\n";
}
