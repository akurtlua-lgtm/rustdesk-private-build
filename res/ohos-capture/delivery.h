#pragma once
#include "frame.h"
#include <functional>
#include <mutex>

namespace rustdesk::capture {
// Consumers are synchronous, must not re-enter or throw, and own any retained frame.
class Delivery {
public:
    using Consumer = std::function<bool(Frame)>;
    using Closed = std::function<void(int)>;
    Delivery(Consumer consumer, Closed closed);
    void authorize();
    void publish(Frame frame);
    void close(int reason);
    bool is_closed();
private:
    std::mutex mutex_;
    Consumer consumer_;
    Closed closed_callback_;
    bool authorized_ = false;
    bool closed_ = false;
};
}
