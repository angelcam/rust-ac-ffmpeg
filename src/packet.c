#include <libavcodec/avcodec.h>
#include <libavutil/buffer.h>

AVPacket* ffw_packet_alloc() {
    return av_packet_alloc();
}

AVPacket* ffw_packet_new(int size) {
    AVPacket* packet = av_packet_alloc();
    if (packet == NULL) {
        return NULL;
    }

    if (av_new_packet(packet, size) != 0) {
        goto err;
    }

    return packet;

err:
    av_packet_free(&packet);

    return NULL;
}

AVPacket* ffw_packet_clone(const AVPacket* src) {
    return av_packet_clone(src);
}

void ffw_packet_free(AVPacket* packet) {
    av_packet_free(&packet);
}

int64_t ffw_packet_get_pts(const AVPacket* packet) {
    return packet->pts;
}

void ffw_packet_set_pts(AVPacket* packet, int64_t pts) {
    packet->pts = pts;
}

int64_t ffw_packet_get_dts(const AVPacket* packet) {
    return packet->dts;
}

void ffw_packet_set_dts(AVPacket* packet, int64_t dts) {
    packet->dts = dts;
}

int ffw_packet_get_stream_index(const AVPacket* packet) {
    return packet->stream_index;
}

void ffw_packet_set_stream_index(AVPacket* packet, int index) {
    packet->stream_index = index;
}

int ffw_packet_is_key(const AVPacket* packet) {
    return packet->flags & AV_PKT_FLAG_KEY;
}

void ffw_packet_set_key(AVPacket* packet, int key) {
    if (key) {
        packet->flags |= AV_PKT_FLAG_KEY;
    } else {
        packet->flags &= ~AV_PKT_FLAG_KEY;
    }
}

int ffw_packet_get_size(const AVPacket* packet) {
    return packet->size;
}

uint8_t* ffw_packet_get_data(AVPacket* packet) {
    return packet->data;
}

int ffw_packet_is_writable(const AVPacket* packet) {
    // XXX: There is no av_packet_is_writable() function. The following check
    // has been copied from the av_packet_make_writable() function.
    return packet->buf && av_buffer_is_writable(packet->buf);
}

int ffw_packet_make_writable(AVPacket* packet) {
    return av_packet_make_writable(packet);
}
