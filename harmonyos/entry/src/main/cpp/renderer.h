#ifndef RUSTDESK_OHOS_RENDERER_H
#define RUSTDESK_OHOS_RENDERER_H
#include "rustdesk_abi.h"
#include <ace/xcomponent/native_interface_xcomponent.h>
#include <string>
namespace rustdesk {
void attach_surface(OH_NativeXComponent* component);
void select_session(const std::string& session);
void destroy_renderer();
int paint(const char* session, uint32_t display, const uint8_t* rgba, size_t length,
          uint32_t width, uint32_t height, size_t stride);
}
#endif
