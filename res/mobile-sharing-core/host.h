#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Extension-only initialization before Config access; existing app directories cannot change. */
int32_t rd_mobile_host_initialize(const uint8_t* directory, size_t length);
/* Explicit local user action, minimum 8 characters, refuses while a sharing scope exists. */
int32_t rd_mobile_host_set_password(const uint8_t* password, size_t length);
/* Strict server-only JSON schema; never transfers account tokens/passwords/device private keys. */
int32_t rd_mobile_host_configure_servers(const uint8_t* data, size_t length);
/* Required buffer size includes NUL; null/too-small output writes nothing. Private app storage only. */
size_t rd_mobile_host_servers(uint8_t* output, size_t capacity);
size_t rd_mobile_host_id(uint8_t* output, size_t capacity);
/* iOS legacy extension only: secret policy bytes, device-local shared Keychain only.
 * Never store in app-group files/logs or send with account/API requests. */
size_t rd_mobile_host_legacy_policy(uint8_t* output, size_t capacity);
int32_t rd_mobile_host_legacy_import(const uint8_t* data, size_t length);
/* Callbacks borrow only for the call; BGRA is copied. Requires current OS consent and
 * configured permanent device password/password-only approval; no account token is accepted. */
uint64_t rd_mobile_host_begin(const uint8_t* pixels, size_t length, uint32_t width, uint32_t height, size_t stride);
int32_t rd_mobile_host_frame(uint64_t token, const uint8_t* pixels, size_t length, uint32_t width, uint32_t height, size_t stride);
int32_t rd_mobile_host_stop(uint64_t token);
/* Native dedicated thread only. Owns the explicitly authorized mobile process runtime.
 * Stop first; wait for run to return before release. No nested runtime or callback/UI blocking. */
int32_t rd_mobile_host_run(uint64_t token);
int32_t rd_mobile_host_release(uint64_t token);
/* 0 absent, 1 worker pending, 2 registering, 3 ready, 4 stopping, 5 stopped, 6 failed.
 * Frame delivery stopping is independent of OS capture stopping; the owner must await both. */
int32_t rd_mobile_host_state(uint64_t token);
#ifdef __cplusplus
}
#endif
