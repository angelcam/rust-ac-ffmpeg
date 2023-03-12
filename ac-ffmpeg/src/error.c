#include <libavutil/common.h>
#include <libavutil/error.h>

int ffw_error_again() {
    return AVERROR(EAGAIN);
}

int ffw_error_eof() {
    return AVERROR_EOF;
}

int ffw_error_would_block() {
    return AVERROR(EWOULDBLOCK);
}

int ffw_error_unknown() {
    return AVERROR_UNKNOWN;
}

int ffw_error_from_posix(int error) {
    return AVERROR(error);
}

int ffw_error_to_posix(int error) {
    return AVUNERROR(error);
}

void ffw_error_get_error_string(int error, char* buffer, size_t buffer_size) {
    av_strerror(error, buffer, buffer_size);

    // make sure that the string ends with null character (ffmpeg does not
    // guarantee that)
    buffer[buffer_size - 1] = 0;
}
