#include "delivery.h"
#include <utility>

namespace rustdesk::capture {
Delivery::Delivery(Consumer consumer, Closed closed)
    : consumer_(std::move(consumer)), closed_callback_(std::move(closed)) {}
void Delivery::authorize() {
    std::lock_guard<std::mutex> lock(mutex_);
    if (!closed_) authorized_ = true;
}
void Delivery::publish(Frame frame) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_ || !authorized_) return;
    if (!consumer_ || !frame.pixels || !consumer_(std::move(frame))) {
        closed_ = true;
        authorized_ = false;
        if (closed_callback_) closed_callback_(-1);
    }
}
void Delivery::close(int reason) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) return;
    closed_ = true;
    authorized_ = false;
    if (closed_callback_) closed_callback_(reason);
}
bool Delivery::is_closed() {
    std::lock_guard<std::mutex> lock(mutex_);
    return closed_;
}
}
