#ifndef RUSTDESK_OHOS_ABI_H
#define RUSTDESK_OHOS_ABI_H
#include <cstddef>
#include <cstdint>
// Borrowed arguments/data last only for the call. Callbacks must not re-enter Rust.
// Setting both callbacks to nullptr waits for all borrowed callbacks to finish.
extern "C" {
using RdEvent = int (*)(const uint8_t*, size_t);
using RdRgba = int (*)(const char*, uint32_t, const uint8_t*, size_t, uint32_t, uint32_t, size_t);
int rd_ohos_init(const char*, const char*);
void rd_ohos_set_callbacks(RdEvent, RdRgba);
char* rd_ohos_version();
// Blocking HTTPS account API, called only by a NAPI async worker.
char* rd_ohos_account_request(const char*, const char*, const char*);
void rd_ohos_free_string(char*);
char* rd_ohos_connect(const char*, const char*, bool);
int rd_ohos_login(const char*, const char*, const char*, const char*, bool);
int rd_ohos_2fa(const char*, const char*, bool);
int rd_ohos_close(const char*);
void rd_ohos_close_all();
int rd_ohos_mouse(const char*, int, int, int, uint32_t);
int rd_ohos_key(const char*, const char*, uint32_t);
int rd_ohos_text(const char*, const char*);
int rd_ohos_view_only(const char*, bool);
}
#endif
