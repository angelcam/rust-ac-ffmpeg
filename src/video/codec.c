#include<libavcodec/avcodec.h>

typedef struct Decoder {
    struct AVCodecContext* cc;
    struct AVFrame* frame;
} Decoder;

Decoder* ffw_decoder_new(const char* codec);
int ffw_decoder_push_packet(Decoder* decoder, const AVPacket* packet);
int ffw_decoder_take_frame(Decoder* decoder, AVFrame** frame);
void ffw_decoder_free(Decoder* decoder);

Decoder* ffw_decoder_new(const char* codec) {
    AVCodec* decoder = avcodec_find_decoder_by_name(codec);
    if (decoder == NULL) {
        return NULL;
    }

    Decoder* res = malloc(sizeof(Decoder));
    if (res == NULL) {
        return NULL;
    }

    res->cc = NULL;
    res->frame = NULL;

    res->cc = avcodec_alloc_context3(decoder);
    if (res->cc == NULL) {
        goto err;
    }

    if (avcodec_open2(res->cc, decoder, NULL) != 0) {
        goto err;
    }

    res->frame = av_frame_alloc();
    if (res->frame == NULL) {
        goto err;
    }

    return res;

err:
    ffw_decoder_free(res);

    return NULL;
}

int ffw_decoder_push_packet(Decoder* decoder, const AVPacket* packet) {
    int ret = avcodec_send_packet(decoder->cc, packet);

    if (ret == 0 || ret == AVERROR_EOF) {
        return 1;
    } else if (ret == AVERROR(EAGAIN)) {
        return 0;
    } else {
        return ret;
    }
}

int ffw_decoder_take_frame(Decoder* decoder, AVFrame** frame) {
    int ret = avcodec_receive_frame(decoder->cc, decoder->frame);

    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
        return 0;
    } else if (ret < 0) {
        return ret;
    }

    *frame = av_frame_clone(decoder->frame);

    return 1;
}

void ffw_decoder_free(Decoder* decoder) {
    if (decoder == NULL) {
        return;
    }

    av_frame_free(&decoder->frame);
    avcodec_free_context(&decoder->cc);
    free(decoder);
}

typedef struct Encoder {
    struct AVCodecContext* cc;
    struct AVCodec* codec;
    struct AVPacket* packet;
} Encoder;

Encoder* ffw_encoder_new(const char* codec);

void ffw_encoder_set_bit_rate(Encoder* encoder, int bit_rate);
void ffw_encoder_set_pixel_format(Encoder* encoder, int format);
void ffw_encoder_set_width(Encoder* encoder, int width);
void ffw_encoder_set_height(Encoder* encoder, int height);
void ffw_encoder_set_time_base(Encoder* encoder, int num, int den);
int ffw_encoder_open(Encoder* encoder);
int ffw_encoder_push_frame(Encoder* encoder, const AVFrame* frame);
int ffw_encoder_take_packet(Encoder* encoder, AVPacket** packet);
void ffw_encoder_free(Encoder* encoder);

Encoder* ffw_encoder_new(const char* codec) {
    AVCodec* encoder = avcodec_find_encoder_by_name(codec);
    if (encoder == NULL) {
        return NULL;
    }

    Encoder* res = malloc(sizeof(Encoder));
    if (res == NULL) {
        return NULL;
    }

    res->codec = encoder;
    res->cc = NULL;
    res->packet = NULL;

    res->cc = avcodec_alloc_context3(encoder);
    if (res->cc == NULL) {
        goto err;
    }

    res->packet = av_packet_alloc();
    if (res->packet == NULL) {
        goto err;
    }

    return res;

err:
    ffw_encoder_free(res);

    return NULL;
}

void ffw_encoder_set_bit_rate(Encoder* encoder, int bit_rate) {
    encoder->cc->bit_rate = bit_rate;
}

void ffw_encoder_set_pixel_format(Encoder* encoder, int format) {
    encoder->cc->pix_fmt = format;
}

void ffw_encoder_set_width(Encoder* encoder, int width) {
    encoder->cc->width = width;
}

void ffw_encoder_set_height(Encoder* encoder, int height) {
    encoder->cc->height = height;
}

void ffw_encoder_set_time_base(Encoder* encoder, int num, int den) {
    encoder->cc->time_base.num = num;
    encoder->cc->time_base.den = den;
}

int ffw_encoder_open(Encoder* encoder) {
    return avcodec_open2(encoder->cc, encoder->codec, NULL);
}

int ffw_encoder_push_frame(Encoder* encoder, const AVFrame* frame) {
    int ret = avcodec_send_frame(encoder->cc, frame);

    if (ret == 0 || ret == AVERROR_EOF) {
        return 1;
    } else if (ret == AVERROR(EAGAIN)) {
        return 0;
    } else {
        return ret;
    }
}

int ffw_encoder_take_packet(Encoder* encoder, AVPacket** packet) {
    int ret = avcodec_receive_packet(encoder->cc, encoder->packet);

    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
        return 0;
    } else if (ret < 0) {
        return ret;
    }

    *packet = av_packet_clone(encoder->packet);

    return 1;
}

void ffw_encoder_free(Encoder* encoder) {
    if (encoder == NULL) {
        return;
    }

    av_packet_free(&encoder->packet);
    avcodec_free_context(&encoder->cc);
    free(encoder);
}
