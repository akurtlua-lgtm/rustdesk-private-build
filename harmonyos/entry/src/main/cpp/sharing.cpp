#include "sharing.h"
#include <algorithm>
#include <atomic>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <string>
#include <thread>

#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
#include "capture.h"
#include "host.h"

namespace {
class Sharing {
public:
    std::shared_ptr<rustdesk::capture::Delivery> delivery;
    std::unique_ptr<rustdesk::capture::Capture> capture;
    std::mutex control;
    std::thread worker;
    std::atomic<bool> done{false};
    std::atomic<int> error{0};
    std::atomic<int> system_error{0};

    bool frame(rustdesk::capture::Frame frame) {
        std::lock_guard<std::mutex> lock(event_mutex_);
        if (closed_) return false;
        const auto size = frame.stride * frame.height;
        if (token_ == 0) {
            token_ = rd_mobile_host_begin(frame.pixels.get(), size,
                static_cast<std::uint32_t>(frame.width), static_cast<std::uint32_t>(frame.height), frame.stride);
            event_.notify_all();
            if (token_ == 0) error.store(-10);
            return token_ != 0;
        }
        return rd_mobile_host_frame(token_, frame.pixels.get(), size,
            static_cast<std::uint32_t>(frame.width), static_cast<std::uint32_t>(frame.height), frame.stride) == 0;
    }
    void close() {
        std::uint64_t token;
        {
            std::lock_guard<std::mutex> lock(event_mutex_);
            closed_ = true;
            token = token_;
        }
        if (token != 0) rd_mobile_host_stop(token);
        event_.notify_all();
    }
    void run() {
        std::uint64_t token;
        {
            std::unique_lock<std::mutex> lock(event_mutex_);
            event_.wait(lock, [&] { return closed_ || token_ != 0; });
            token = token_;
        }
        int result = 0;
        if (token != 0) {
            result = rd_mobile_host_run(token);
            rd_mobile_host_stop(token);
            const auto released = rd_mobile_host_release(token);
            if (result == 0 && released != 0) result = released;
        }
        close();
        {
            std::lock_guard<std::mutex> lock(control);
            system_error.store(capture->stop());
        }
        if (result != 0) { int empty = 0; error.compare_exchange_strong(empty, result); }
        done.store(true);
    }
    int stop_and_join() {
        close();
        std::lock_guard<std::mutex> join(join_mutex_);
        if (worker.joinable()) {
            worker.join();
        } else if (capture) {
            std::lock_guard<std::mutex> lock(control);
            system_error.store(capture->stop());
            done.store(true);
        }
        return system_error.load();
    }
    int phase() {
        if (done.load()) return error.load() != 0 || system_error.load() != 0 ? 6 : 5;
        std::lock_guard<std::mutex> lock(event_mutex_);
        if (closed_) return 4;
        return token_ == 0 ? 1 : rd_mobile_host_state(token_);
    }
private:
    std::mutex event_mutex_;
    std::condition_variable event_;
    std::mutex join_mutex_;
    std::uint64_t token_ = 0;
    bool closed_ = false;
};

struct State {
    std::mutex mutex;
    std::shared_ptr<Sharing> active;
    ~State() { if (active) active->stop_and_join(); }
};
State& state() { static State value; return value; }
void shutdown(void*) {
    std::shared_ptr<Sharing> sharing;
    { std::lock_guard<std::mutex> lock(state().mutex); sharing = state().active; }
    if (sharing) sharing->stop_and_join();
}
}
#endif

namespace {
napi_value number(napi_env env, int value) {
    napi_value result = nullptr;
    if (napi_create_int32(env, value, &result) != napi_ok) napi_throw_error(env, nullptr, "Cannot create sharing result");
    return result;
}
napi_value supported(napi_env env, napi_callback_info) {
    napi_value result = nullptr;
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    const bool enabled = true;
#else
    const bool enabled = false;
#endif
    if (napi_get_boolean(env, enabled, &result) != napi_ok) napi_throw_error(env, nullptr, "Cannot read sharing capability");
    return result;
}
napi_value password(napi_env env, napi_callback_info info) try {
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    napi_value argument = nullptr;
    std::size_t count = 1, length = 0;
    if (napi_get_cb_info(env, info, &count, &argument, nullptr, nullptr) != napi_ok || count != 1
        || napi_get_value_string_utf8(env, argument, nullptr, 0, &length) != napi_ok || length > 128) return number(env, -1);
    std::string value(length + 1, '\0');
    struct Wipe {
        std::string& value;
        ~Wipe() { for (volatile char& byte : value) byte = 0; }
    } wipe{value};
    if (napi_get_value_string_utf8(env, argument, value.data(), value.size(), &length) != napi_ok) return number(env, -1);
    const auto result = rd_mobile_host_set_password(reinterpret_cast<const std::uint8_t*>(value.data()), length);
    return number(env, result);
#else
    (void)info; return number(env, -1);
#endif
} catch (...) { return number(env, -1); }
napi_value start(napi_env env, napi_callback_info info) {
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    napi_value arguments[2]{};
    std::size_t count = 2;
    std::int32_t width = 0, height = 0;
    if (napi_get_cb_info(env, info, &count, arguments, nullptr, nullptr) != napi_ok || count != 2
        || napi_get_value_int32(env, arguments[0], &width) != napi_ok
        || napi_get_value_int32(env, arguments[1], &height) != napi_ok || width <= 0 || height <= 0) return number(env, -1);
    try {
        std::lock_guard<std::mutex> lock(state().mutex);
        if (state().active) return number(env, -2);
        auto sharing = std::make_shared<Sharing>();
        std::weak_ptr<Sharing> weak = sharing;
        sharing->delivery = std::make_shared<rustdesk::capture::Delivery>(
            [weak](rustdesk::capture::Frame frame) { auto owner = weak.lock(); return owner && owner->frame(std::move(frame)); },
            [weak](int) { if (auto owner = weak.lock()) owner->close(); });
        int error = 0;
        sharing->capture = rustdesk::capture::Capture::create(width, height, sharing->delivery, error);
        if (!sharing->capture) return number(env, error);
        state().active = sharing;
        try {
            sharing->worker = std::thread([owner = sharing.get()] { owner->run(); });
        } catch (...) {
            sharing->error.store(-1);
            sharing->stop_and_join();
            return number(env, -1);
        }
        std::lock_guard<std::mutex> control(sharing->control);
        const auto result = sharing->capture->start();
        if (result != 0) sharing->error.store(result);
        return number(env, result);
    } catch (...) { return number(env, -1); }
#else
    (void)info; return number(env, -1);
#endif
}

struct StopJob {
    napi_deferred deferred = nullptr;
    napi_async_work work = nullptr;
    int result = 0;
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    std::shared_ptr<Sharing> sharing;
#endif
};
napi_value stop(napi_env env, napi_callback_info) try {
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    std::shared_ptr<Sharing> active;
    { std::lock_guard<std::mutex> lock(state().mutex); active = state().active; }
    if (active) active->close();
#endif
    auto job = std::make_unique<StopJob>();
    napi_value promise = nullptr, label = nullptr;
    if (napi_create_promise(env, &job->deferred, &promise) != napi_ok
        || napi_create_string_utf8(env, "stop-sharing", NAPI_AUTO_LENGTH, &label) != napi_ok) {
        napi_throw_error(env, nullptr, "Cannot create sharing cleanup request"); return nullptr;
    }
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    job->sharing = active;
#endif
    auto execute = [](napi_env, void* data) {
        auto* job = static_cast<StopJob*>(data);
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
        if (job->sharing) {
            job->result = job->sharing->stop_and_join();
            if (job->result == 0) {
                std::lock_guard<std::mutex> lock(state().mutex);
                if (state().active == job->sharing) state().active.reset();
            }
        }
#else
        (void)job;
#endif
    };
    auto complete = [](napi_env env, napi_status status, void* data) {
        std::unique_ptr<StopJob> job(static_cast<StopJob*>(data));
        napi_resolve_deferred(env, job->deferred, number(env, status == napi_ok ? job->result : -1));
        napi_delete_async_work(env, job->work);
    };
    if (napi_create_async_work(env, nullptr, label, execute, complete, job.get(), &job->work) != napi_ok) {
        napi_throw_error(env, nullptr, "Cannot schedule sharing cleanup"); return nullptr;
    }
    if (napi_queue_async_work(env, job->work) != napi_ok) {
        napi_delete_async_work(env, job->work);
        napi_throw_error(env, nullptr, "Cannot queue sharing cleanup"); return nullptr;
    }
    job.release();
    return promise;
} catch (...) { napi_throw_error(env, nullptr, "Cannot prepare sharing cleanup"); return nullptr; }

napi_value status(napi_env env, napi_callback_info) {
    int phase = 0, error = 0;
    std::string id;
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    std::shared_ptr<Sharing> sharing;
    { std::lock_guard<std::mutex> lock(state().mutex); sharing = state().active; }
    if (sharing) {
        phase = sharing->phase();
        error = sharing->system_error.load() != 0 ? sharing->system_error.load() : sharing->error.load();
    }
    std::uint8_t buffer[256]{};
    const auto needed = rd_mobile_host_id(buffer, sizeof(buffer));
    if (needed > 0 && needed <= sizeof(buffer)) id.assign(reinterpret_cast<const char*>(buffer), needed - 1);
#endif
    napi_value result = nullptr, text = nullptr;
    if (napi_create_object(env, &result) != napi_ok
        || napi_create_string_utf8(env, id.data(), id.size(), &text) != napi_ok
        || napi_set_named_property(env, result, "phase", number(env, phase)) != napi_ok
        || napi_set_named_property(env, result, "errorCode", number(env, error)) != napi_ok
        || napi_set_named_property(env, result, "deviceId", text) != napi_ok) {
        napi_throw_error(env, nullptr, "Cannot read sharing state"); return nullptr;
    }
    return result;
}
}

napi_value register_sharing(napi_env env, napi_value exports) {
#ifdef RD_ENABLE_MOBILE_SCREEN_SHARING
    if (napi_add_env_cleanup_hook(env, shutdown, nullptr) != napi_ok) return nullptr;
#endif
    const napi_property_descriptor properties[] = {
        {"sharingSupported", nullptr, supported, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sharingPassword", nullptr, password, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sharingStart", nullptr, start, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sharingStop", nullptr, stop, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"sharingStatus", nullptr, status, nullptr, nullptr, nullptr, napi_default, nullptr}
    };
    if (napi_define_properties(env, exports, sizeof(properties) / sizeof(properties[0]), properties) != napi_ok) return nullptr;
    return exports;
}
