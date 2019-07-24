#include <libavformat/avio.h>

typedef int read_packet_t(void*, uint8_t*, int);
typedef int write_packet_t(void*, uint8_t*, int);
typedef int64_t seek_t(void*, int64_t, int);

int ffw_io_is_avseek_size(int whence) {
    return whence & AVSEEK_SIZE;
}

AVIOContext * ffw_io_context_new(
    int buffer_size,
    int write_flag,
    void* opaque,
    read_packet_t* read_packet,
    write_packet_t* write_packet,
    seek_t* seek) {
    unsigned char* buffer = av_malloc(buffer_size);
    if (buffer == NULL) {
        return NULL;
    }

    AVIOContext* context = avio_alloc_context(
        buffer,
        buffer_size,
        write_flag,
        opaque,
        read_packet,
        write_packet,
        seek);

    if (context == NULL) {
        goto err;
    }

    return context;

err:
    av_free(buffer);

    return NULL;
}

void ffw_io_context_free(AVIOContext* context) {
    if (context) {
        av_freep(&context->buffer);
    }

    avio_context_free(&context);
}
