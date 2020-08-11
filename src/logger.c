#include <stdio.h>
#include <libavutil/log.h>

#ifdef __GNUC__
#define UNUSED(x) UNUSED_ ## x __attribute__((__unused__))
#else
#define UNUSED(x) UNUSED_ ## x
#endif

static void (*rust_callback)(int, const char*) = NULL;

static void log_callback(void* UNUSED(ptr), int level, const char* fmt, va_list vl) {
    char buffer[4096];

    memset(buffer, 0, sizeof(buffer));
    vsnprintf(buffer, sizeof(buffer), fmt, vl);
    (*rust_callback)(level, buffer);
}

void ffw_set_log_callback(void (*callback)(int, const char*)) {
    rust_callback = callback;
    av_log_set_callback(log_callback);
}
