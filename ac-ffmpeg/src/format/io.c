#include <libavformat/avio.h>
#include <libavutil/mem.h>

typedef int read_packet_t(void*, uint8_t*, int);

#if LIBAVFORMAT_VERSION_MAJOR < 61
typedef int write_packet_t(void*, uint8_t*, int);
#else
typedef int write_packet_t(void*, const uint8_t*, int);
#endif

typedef int64_t seek_t(void*, int64_t, int);

int ffw_io_whence_to_seek_mode(int whence) {
    if (whence & AVSEEK_SIZE) {
        return 0;
    }

    switch (whence) {
        case SEEK_SET: return 1;
        case SEEK_CUR: return 2;
        case SEEK_END: return 3;
        default: return -1;
    }
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
