#include "renderer.h"
#include <EGL/egl.h>
#include <GLES2/gl2.h>
#include <limits>
#include <mutex>
namespace rustdesk {
namespace {
std::mutex render_mutex;
std::string selected_session;
void* native_window = nullptr;
EGLDisplay egl_display = EGL_NO_DISPLAY;
EGLSurface surface = EGL_NO_SURFACE;
EGLContext context = EGL_NO_CONTEXT;
GLuint program = 0;
GLuint texture = 0;
GLint position_location = -1;
GLint uv_location = -1;
GLint maximum_texture = 0;

void reset() {
    if (context != EGL_NO_CONTEXT && surface != EGL_NO_SURFACE &&
        eglMakeCurrent(egl_display, surface, surface, context)) {
        glDeleteTextures(1, &texture);
        glDeleteProgram(program);
        eglMakeCurrent(egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    }
    if (context != EGL_NO_CONTEXT) eglDestroyContext(egl_display, context);
    if (surface != EGL_NO_SURFACE) eglDestroySurface(egl_display, surface);
    context = EGL_NO_CONTEXT;
    surface = EGL_NO_SURFACE;
    program = texture = 0;
    // The default display can also be owned by ArkUI; do not terminate it here.
}

GLuint shader(GLenum kind, const char* source) {
    const GLuint id = glCreateShader(kind);
    glShaderSource(id, 1, &source, nullptr);
    glCompileShader(id);
    GLint compiled = GL_FALSE;
    glGetShaderiv(id, GL_COMPILE_STATUS, &compiled);
    if (compiled != GL_TRUE) { glDeleteShader(id); return 0; }
    return id;
}

bool prepare() {
    egl_display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (egl_display == EGL_NO_DISPLAY || !eglInitialize(egl_display, nullptr, nullptr) ||
        !eglBindAPI(EGL_OPENGL_ES_API)) return false;
    const EGLint attributes[] = {EGL_SURFACE_TYPE, EGL_WINDOW_BIT, EGL_RENDERABLE_TYPE,
        EGL_OPENGL_ES2_BIT, EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8, EGL_NONE};
    EGLConfig config = nullptr;
    EGLint count = 0;
    if (!eglChooseConfig(egl_display, attributes, &config, 1, &count) || count != 1) return false;
    surface = eglCreateWindowSurface(egl_display, config, reinterpret_cast<EGLNativeWindowType>(native_window), nullptr);
    const EGLint context_attributes[] = {EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE};
    context = eglCreateContext(egl_display, config, EGL_NO_CONTEXT, context_attributes);
    if (surface == EGL_NO_SURFACE || context == EGL_NO_CONTEXT ||
        !eglMakeCurrent(egl_display, surface, surface, context)) { reset(); return false; }
    const GLuint vertex = shader(GL_VERTEX_SHADER,
        "attribute vec2 position; attribute vec2 uv; varying vec2 tex; void main(){tex=uv;gl_Position=vec4(position,0.,1.);}");
    const GLuint fragment = shader(GL_FRAGMENT_SHADER,
        "precision mediump float; varying vec2 tex; uniform sampler2D image; void main(){gl_FragColor=texture2D(image,tex);}");
    if (vertex == 0 || fragment == 0) {
        glDeleteShader(vertex); glDeleteShader(fragment); reset(); return false;
    }
    program = glCreateProgram();
    glAttachShader(program, vertex); glAttachShader(program, fragment); glLinkProgram(program);
    glDeleteShader(vertex); glDeleteShader(fragment);
    GLint linked = GL_FALSE;
    glGetProgramiv(program, GL_LINK_STATUS, &linked);
    position_location = glGetAttribLocation(program, "position");
    uv_location = glGetAttribLocation(program, "uv");
    if (linked != GL_TRUE || position_location < 0 || uv_location < 0) { reset(); return false; }
    glUseProgram(program);
    glUniform1i(glGetUniformLocation(program, "image"), 0);
    glGenTextures(1, &texture);
    glBindTexture(GL_TEXTURE_2D, texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glGetIntegerv(GL_MAX_TEXTURE_SIZE, &maximum_texture);
    eglMakeCurrent(egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    return true;
}
void created(OH_NativeXComponent*, void* window) {
    const std::lock_guard<std::mutex> guard(render_mutex);
    reset(); native_window = window;
}
void changed(OH_NativeXComponent*, void*) {}
void destroyed(OH_NativeXComponent*, void* window) {
    const std::lock_guard<std::mutex> guard(render_mutex);
    if (native_window == window) { reset(); native_window = nullptr; }
}
OH_NativeXComponent_Callback callbacks{created, changed, destroyed, nullptr};
}
void attach_surface(OH_NativeXComponent* component) {
    if (OH_NativeXComponent_RegisterCallback(component, &callbacks) != OH_NATIVEXCOMPONENT_RESULT_SUCCESS) {
        // No surface is made usable if registration fails.
        destroy_renderer();
    }
}
void select_session(const std::string& session) {
    const std::lock_guard<std::mutex> guard(render_mutex);
    selected_session = session;
}
void destroy_renderer() {
    const std::lock_guard<std::mutex> guard(render_mutex);
    reset(); native_window = nullptr; selected_session.clear();
}
int paint(const char* session, uint32_t, const uint8_t* rgba, size_t length,
          uint32_t width, uint32_t height, size_t stride) {
    if (!session || !rgba || width == 0 || height == 0 ||
        width > static_cast<uint32_t>(std::numeric_limits<GLsizei>::max()) ||
        height > static_cast<uint32_t>(std::numeric_limits<GLsizei>::max()) ||
        stride != static_cast<size_t>(width) * 4 || height > length / stride) return -1;
    const std::lock_guard<std::mutex> guard(render_mutex);
    if (selected_session != session || !native_window) return -1;
    if (context == EGL_NO_CONTEXT && !prepare()) return -2;
    if (width > static_cast<uint32_t>(maximum_texture) || height > static_cast<uint32_t>(maximum_texture) ||
        !eglMakeCurrent(egl_display, surface, surface, context)) return -2;
    EGLint viewport_width = 0, viewport_height = 0;
    const bool dimensions = eglQuerySurface(egl_display, surface, EGL_WIDTH, &viewport_width) &&
        eglQuerySurface(egl_display, surface, EGL_HEIGHT, &viewport_height);
    glViewport(0, 0, viewport_width, viewport_height);
    glUseProgram(program);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, texture);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, static_cast<GLsizei>(width), static_cast<GLsizei>(height),
        0, GL_RGBA, GL_UNSIGNED_BYTE, rgba);
    constexpr GLfloat vertices[] = {-1, -1, 0, 1, 1, -1, 1, 1, -1, 1, 0, 0, 1, 1, 1, 0};
    glVertexAttribPointer(static_cast<GLuint>(position_location), 2, GL_FLOAT, GL_FALSE, 4 * sizeof(GLfloat), vertices);
    glVertexAttribPointer(static_cast<GLuint>(uv_location), 2, GL_FLOAT, GL_FALSE, 4 * sizeof(GLfloat), vertices + 2);
    glEnableVertexAttribArray(static_cast<GLuint>(position_location));
    glEnableVertexAttribArray(static_cast<GLuint>(uv_location));
    glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    const bool good = dimensions && glGetError() == GL_NO_ERROR && eglSwapBuffers(egl_display, surface);
    eglMakeCurrent(egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
    return good ? 0 : -2;
}
}
