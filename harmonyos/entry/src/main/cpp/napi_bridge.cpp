#include "renderer.h"
#include <napi/native_api.h>
#include <array>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
namespace {
std::mutex event_mutex;
napi_threadsafe_function event_function = nullptr;
napi_env owner_env = nullptr;

napi_value fail(napi_env env, const char* message) {
    napi_throw_type_error(env, nullptr, message);
    return nullptr;
}
napi_value number(napi_env env, int value) {
    napi_value result = nullptr;
    if (napi_create_int32(env, value, &result) != napi_ok) return fail(env, "Cannot return native result");
    return result;
}
napi_value string(napi_env env, const char* value) {
    napi_value result = nullptr;
    if (!value || napi_create_string_utf8(env, value, NAPI_AUTO_LENGTH, &result) != napi_ok)
        return fail(env, "Native operation failed");
    return result;
}
struct RustStringDeleter { void operator()(char* value) const { rd_ohos_free_string(value); } };
using RustString = std::unique_ptr<char, RustStringDeleter>;

struct Arguments {
    napi_env env;
    std::array<napi_value, 6> values{};
    size_t count = values.size();
    bool valid = false;
    Arguments(napi_env environment, napi_callback_info info, size_t expected) : env(environment) {
        valid = napi_get_cb_info(env, info, &count, values.data(), nullptr, nullptr) == napi_ok && count == expected;
    }
    bool text(size_t index, std::string& output, size_t maximum = 65536) const {
        size_t size = 0;
        if (!valid || napi_get_value_string_utf8(env, values[index], nullptr, 0, &size) != napi_ok || size > maximum)
            return false;
        output.resize(size + 1);
        size_t written = 0;
        if (napi_get_value_string_utf8(env, values[index], output.data(), output.size(), &written) != napi_ok ||
            written != size) return false;
        output.resize(size);
        return output.find('\0') == std::string::npos;
    }
    bool integer(size_t index, int32_t& output) const {
        return valid && napi_get_value_int32(env, values[index], &output) == napi_ok;
    }
    bool boolean(size_t index, bool& output) const {
        return valid && napi_get_value_bool(env, values[index], &output) == napi_ok;
    }
};

void call_js(napi_env env, napi_value callback, void*, void* raw) {
    const std::unique_ptr<std::string> data(static_cast<std::string*>(raw));
    if (!env || !callback || !data) return;
    napi_value argument = nullptr, receiver = nullptr, result = nullptr;
    if (napi_create_string_utf8(env, data->data(), data->size(), &argument) != napi_ok ||
        napi_get_undefined(env, &receiver) != napi_ok) return;
    // Any JS exception stays pending for the ArkTS event handler; never log its payload.
    napi_call_function(env, receiver, callback, 1, &argument, &result);
}
int event(const uint8_t* bytes, size_t length) noexcept {
    if (!bytes || length > 4 * 1024 * 1024) return -1;
    auto data = std::make_unique<std::string>(reinterpret_cast<const char*>(bytes), length);
    const std::lock_guard<std::mutex> guard(event_mutex);
    if (!event_function) return 0;
    const napi_status status = napi_call_threadsafe_function(event_function, data.get(), napi_tsfn_nonblocking);
    if (status == napi_ok) data.release();
    return status == napi_ok ? 0 : -1;
}
void cleanup(void*) {
    // Rust waits for synchronous, non-reentrant callbacks before their C++ resources are released.
    rd_ohos_set_callbacks(nullptr, nullptr);
    napi_threadsafe_function previous = nullptr;
    {
        const std::lock_guard<std::mutex> guard(event_mutex);
        previous = event_function; event_function = nullptr;
    }
    if (previous) napi_release_threadsafe_function(previous, napi_tsfn_abort);
    rd_ohos_close_all();
    rustdesk::destroy_renderer();
    owner_env = nullptr;
}
napi_value initialize(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 2);
    std::string home, files;
    if (!args.text(0, home, 4096) || !args.text(1, files, 4096)) return fail(env, "Expected app-owned directories");
    if (owner_env && owner_env != env) return fail(env, "Native runtime already belongs to another environment");
    const int result = rd_ohos_init(home.c_str(), files.c_str());
    if (result != 0) return number(env, result);
    if (!owner_env) {
        if (napi_add_env_cleanup_hook(env, cleanup, nullptr) != napi_ok) return fail(env, "Cannot register native cleanup");
        owner_env = env;
    }
    rd_ohos_set_callbacks(event, rustdesk::paint);
    return number(env, 0);
}
napi_value on_event(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 1);
    napi_valuetype kind = napi_undefined;
    if (owner_env != env || !args.valid || napi_typeof(env, args.values[0], &kind) != napi_ok || kind != napi_function)
        return fail(env, "Initialize native runtime before registering an event handler");
    napi_value name = nullptr;
    napi_threadsafe_function next = nullptr;
    if (napi_create_string_utf8(env, "RustDesk events", NAPI_AUTO_LENGTH, &name) != napi_ok ||
        napi_create_threadsafe_function(env, args.values[0], nullptr, name, 64, 1, nullptr, nullptr, nullptr,
            call_js, &next) != napi_ok) return fail(env, "Cannot register native event handler");
    napi_threadsafe_function previous = nullptr;
    {
        const std::lock_guard<std::mutex> guard(event_mutex);
        previous = event_function; event_function = next;
    }
    if (previous) napi_release_threadsafe_function(previous, napi_tsfn_abort);
    return number(env, 0);
}
napi_value version(napi_env env, napi_callback_info) {
    const RustString value(rd_ohos_version());
    return string(env, value.get());
}
struct AccountWork {
    napi_async_work work = nullptr;
    napi_deferred deferred = nullptr;
    std::string operation;
    std::string payload;
    std::string token;
    RustString response;
};
void execute_account(napi_env, void* raw) {
    auto* task = static_cast<AccountWork*>(raw);
    task->response.reset(rd_ohos_account_request(task->operation.c_str(), task->payload.c_str(), task->token.c_str()));
    task->payload.clear(); task->token.clear();
}
void complete_account(napi_env env, napi_status status, void* raw) {
    const std::unique_ptr<AccountWork> task(static_cast<AccountWork*>(raw));
    napi_value result = nullptr;
    if (status == napi_ok && task->response &&
        napi_create_string_utf8(env, task->response.get(), NAPI_AUTO_LENGTH, &result) == napi_ok) {
        napi_resolve_deferred(env, task->deferred, result);
    } else {
        napi_value message = nullptr;
        if (napi_create_string_utf8(env, "Account request failed", NAPI_AUTO_LENGTH, &message) == napi_ok &&
            napi_create_error(env, nullptr, message, &result) == napi_ok)
            napi_reject_deferred(env, task->deferred, result);
    }
    napi_delete_async_work(env, task->work);
}
napi_value account(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 3);
    auto task = std::make_unique<AccountWork>();
    if (owner_env != env || !args.text(0, task->operation, 32) || !args.text(1, task->payload) ||
        !args.text(2, task->token, 4096)) return fail(env, "Invalid account arguments");
    napi_value resource = nullptr, promise = nullptr;
    if (napi_create_string_utf8(env, "RustDesk account", NAPI_AUTO_LENGTH, &resource) != napi_ok ||
        napi_create_promise(env, &task->deferred, &promise) != napi_ok ||
        napi_create_async_work(env, nullptr, resource, execute_account, complete_account, task.get(), &task->work) != napi_ok)
        return fail(env, "Cannot initialize account request");
    if (napi_queue_async_work(env, task->work) != napi_ok) {
        napi_delete_async_work(env, task->work);
        return fail(env, "Cannot queue account request");
    }
    task.release();
    return promise;
}
napi_value connect(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 3);
    std::string peer, password;
    bool relay = false;
    if (owner_env != env || !args.text(0, peer, 1024) || !args.text(1, password) || !args.boolean(2, relay))
        return fail(env, "Expected peer, password and relay preference");
    const RustString id(rd_ohos_connect(peer.c_str(), password.c_str(), relay));
    if (!id) return fail(env, "Unable to create remote session");
    rustdesk::select_session(id.get());
    return string(env, id.get());
}
napi_value login(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 5);
    std::string id, password, user, os_password;
    bool remember = false;
    if (!args.text(0, id, 64) || !args.text(1, password) || !args.text(2, user) || !args.text(3, os_password) ||
        !args.boolean(4, remember)) return fail(env, "Invalid login arguments");
    return number(env, rd_ohos_login(id.c_str(), password.c_str(), user.c_str(), os_password.c_str(), remember));
}
napi_value two_factor(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 3); std::string id, code; bool trust = false;
    if (!args.text(0, id, 64) || !args.text(1, code, 1024) || !args.boolean(2, trust)) return fail(env, "Invalid two-factor arguments");
    return number(env, rd_ohos_2fa(id.c_str(), code.c_str(), trust));
}
napi_value close(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 1); std::string id;
    if (!args.text(0, id, 64)) return fail(env, "Expected session ID");
    rustdesk::select_session("");
    return number(env, rd_ohos_close(id.c_str()));
}
napi_value mouse(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 5); std::string id; int32_t mask = 0, x = 0, y = 0, modifiers = 0;
    if (!args.text(0, id, 64) || !args.integer(1, mask) || !args.integer(2, x) || !args.integer(3, y) ||
        !args.integer(4, modifiers)) return fail(env, "Invalid pointer arguments");
    return number(env, rd_ohos_mouse(id.c_str(), mask, x, y, static_cast<uint32_t>(modifiers)));
}
napi_value key(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 3); std::string id, value; int32_t modifiers = 0;
    if (!args.text(0, id, 64) || !args.text(1, value, 64) || !args.integer(2, modifiers)) return fail(env, "Invalid key arguments");
    return number(env, rd_ohos_key(id.c_str(), value.c_str(), static_cast<uint32_t>(modifiers)));
}
napi_value text(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 2); std::string id, value;
    if (!args.text(0, id, 64) || !args.text(1, value)) return fail(env, "Invalid text arguments");
    return number(env, rd_ohos_text(id.c_str(), value.c_str()));
}
napi_value view_only(napi_env env, napi_callback_info info) {
    Arguments args(env, info, 2); std::string id; bool enabled = true;
    if (!args.text(0, id, 64) || !args.boolean(1, enabled)) return fail(env, "Invalid view-only arguments");
    return number(env, rd_ohos_view_only(id.c_str(), enabled));
}
napi_value module_init(napi_env env, napi_value exports) {
    const napi_property_descriptor properties[] = {
        {"initialize", nullptr, initialize, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"onEvent", nullptr, on_event, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"version", nullptr, version, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"account", nullptr, account, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"connect", nullptr, connect, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"login", nullptr, login, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"twoFactor", nullptr, two_factor, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"close", nullptr, close, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"mouse", nullptr, mouse, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"key", nullptr, key, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"text", nullptr, text, nullptr, nullptr, nullptr, napi_default, nullptr},
        {"viewOnly", nullptr, view_only, nullptr, nullptr, nullptr, napi_default, nullptr}
    };
    if (napi_define_properties(env, exports, sizeof(properties) / sizeof(properties[0]), properties) != napi_ok)
        return fail(env, "Cannot expose native methods");
    bool has_component = false;
    if (napi_has_named_property(env, exports, OH_NATIVE_XCOMPONENT_OBJ, &has_component) == napi_ok && has_component) {
        napi_value object = nullptr; OH_NativeXComponent* component = nullptr;
        if (napi_get_named_property(env, exports, OH_NATIVE_XCOMPONENT_OBJ, &object) != napi_ok ||
            napi_unwrap(env, object, reinterpret_cast<void**>(&component)) != napi_ok || !component)
            return fail(env, "Cannot attach native surface");
        rustdesk::attach_surface(component);
    }
    return exports;
}
napi_module module{1, 0, nullptr, module_init, "entry", nullptr, {nullptr, nullptr, nullptr, nullptr}};
}
extern "C" __attribute__((constructor)) void register_rustdesk_module() { napi_module_register(&module); }
