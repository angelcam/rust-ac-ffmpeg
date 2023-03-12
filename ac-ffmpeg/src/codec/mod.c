#include <libavcodec/avcodec.h>
#include <libavutil/channel_layout.h>

static const AVCodec* ffw_find_codec(const char* name, int type) {
    const AVCodec* codec;
    void* i = NULL;

    if (name == NULL) {
        return NULL;
    }

    while ((codec = av_codec_iterate(&i))) {
        if (codec->type == type) {
            if (strcmp(name, codec->name) == 0) {
                return codec;
            }
        }
    }

    return NULL;
}

AVCodecParameters* ffw_codec_parameters_new(const char* codec_name, int codec_type) {
    AVCodecParameters* res;
    const AVCodec* codec;

    codec = ffw_find_codec(codec_name, codec_type);
    if (codec == NULL) {
        return NULL;
    }

    res = avcodec_parameters_alloc();
    if (res == NULL) {
        return NULL;
    }

    res->codec_type = codec->type;
    res->codec_id = codec->id;

    return res;
}

AVCodecParameters* ffw_audio_codec_parameters_new(const char* codec_name) {
    return ffw_codec_parameters_new(codec_name, AVMEDIA_TYPE_AUDIO);
}

AVCodecParameters* ffw_video_codec_parameters_new(const char* codec_name) {
    return ffw_codec_parameters_new(codec_name, AVMEDIA_TYPE_VIDEO);
}

AVCodecParameters* ffw_subtitle_codec_parameters_new(const char* codec_name) {
    return ffw_codec_parameters_new(codec_name, AVMEDIA_TYPE_SUBTITLE);
}

AVCodecParameters* ffw_codec_parameters_clone(const AVCodecParameters* src) {
    AVCodecParameters* res = avcodec_parameters_alloc();
    if (res == NULL) {
        return NULL;
    }

    if (avcodec_parameters_copy(res, src) < 0) {
        goto err;
    }

    return res;

err:
    avcodec_parameters_free(&res);

    return NULL;
}

int ffw_codec_parameters_is_audio_codec(const AVCodecParameters* params) {
    return params->codec_type == AVMEDIA_TYPE_AUDIO;
}

int ffw_codec_parameters_is_video_codec(const AVCodecParameters* params) {
    return params->codec_type == AVMEDIA_TYPE_VIDEO;
}

int ffw_codec_parameters_is_subtitle_codec(const AVCodecParameters* params) {
    return params->codec_type == AVMEDIA_TYPE_SUBTITLE;
}

const char* ffw_codec_parameters_get_decoder_name(const AVCodecParameters* params) {
    const AVCodec* codec = avcodec_find_decoder(params->codec_id);
    if (!codec) {
        return NULL;
    }

    return codec->name;
}

const char* ffw_codec_parameters_get_encoder_name(const AVCodecParameters* params) {
    const AVCodec* codec = avcodec_find_encoder(params->codec_id);
    if (!codec) {
        return NULL;
    }

    return codec->name;
}

int64_t ffw_codec_parameters_get_bit_rate(const AVCodecParameters* params) {
    return params->bit_rate;
}

int ffw_codec_parameters_get_format(const AVCodecParameters* params) {
    return params->format;
}

int ffw_codec_parameters_get_width(const AVCodecParameters* params) {
    return params->width;
}

int ffw_codec_parameters_get_height(const AVCodecParameters* params) {
    return params->height;
}

int ffw_codec_parameters_get_sample_rate(const AVCodecParameters* params) {
    return params->sample_rate;
}

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
const AVChannelLayout * ffw_codec_parameters_get_channel_layout(const AVCodecParameters* params) {
    return &params->ch_layout;
}
#else
const uint64_t * ffw_codec_parameters_get_channel_layout(const AVCodecParameters* params) {
    return &params->channel_layout;
}
#endif

uint8_t* ffw_codec_parameters_get_extradata(AVCodecParameters* params) {
    return params->extradata;
}

int ffw_codec_parameters_get_extradata_size(const AVCodecParameters* params) {
    return params->extradata_size;
}

void ffw_codec_parameters_set_bit_rate(AVCodecParameters* params, int64_t bit_rate) {
    params->bit_rate = bit_rate;
}

void ffw_codec_parameters_set_format(AVCodecParameters* params, int format) {
    params->format = format;
}

void ffw_codec_parameters_set_width(AVCodecParameters* params, int width) {
    params->width = width;
}

void ffw_codec_parameters_set_height(AVCodecParameters* params, int height) {
    params->height = height;
}

void ffw_codec_parameters_set_sample_rate(AVCodecParameters* params, int sample_rate) {
    params->sample_rate = sample_rate;
}

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
int ffw_codec_parameters_set_channel_layout(AVCodecParameters* params, const AVChannelLayout* channel_layout) {
    return av_channel_layout_copy(&params->ch_layout, channel_layout);
}
#else
int ffw_codec_parameters_set_channel_layout(AVCodecParameters* params, const uint64_t* channel_layout) {
    uint64_t ch_layout = *channel_layout;

    params->channel_layout = ch_layout;
    params->channels = av_get_channel_layout_nb_channels(ch_layout);

    return 0;
}
#endif

int ffw_codec_parameters_set_extradata(AVCodecParameters* params, const uint8_t* extradata, int size) {
    if (params->extradata) {
        av_freep(&params->extradata);
    }

    params->extradata_size = 0;

    if (extradata == NULL || size <= 0) {
        params->extradata = NULL;
    } else {
        params->extradata = av_mallocz(size + AV_INPUT_BUFFER_PADDING_SIZE);
        if (params->extradata == NULL) {
            return -1;
        }

        memcpy(params->extradata, extradata, size);

        params->extradata_size = size;
    }

    return 0;
}

void ffw_codec_parameters_free(AVCodecParameters* params) {
    avcodec_parameters_free(&params);
}

typedef struct Decoder {
    const struct AVCodec* decoder;
    struct AVDictionary* options;
    struct AVCodecContext* cc;
    struct AVFrame* frame;
} Decoder;

Decoder* ffw_decoder_new(const char* codec);
Decoder* ffw_decoder_from_codec_parameters(const AVCodecParameters* params);
int ffw_decoder_set_extradata(Decoder* decoder, const uint8_t* extradata, int size);
int ffw_decoder_set_initial_option(Decoder* decoder, const char* key, const char* value);
int ffw_decoder_open(Decoder* decoder);
int ffw_decoder_push_packet(Decoder* decoder, const AVPacket* packet);
int ffw_decoder_take_frame(Decoder* decoder, AVFrame** frame);
AVCodecParameters* ffw_decoder_get_codec_parameters(const Decoder* decoder);
void ffw_decoder_free(Decoder* decoder);

Decoder* ffw_decoder_new(const char* codec) {
    const AVCodec* decoder = avcodec_find_decoder_by_name(codec);
    if (decoder == NULL) {
        return NULL;
    }

    Decoder* res = malloc(sizeof(Decoder));
    if (res == NULL) {
        return NULL;
    }

    res->decoder = decoder;
    res->options = NULL;
    res->cc = NULL;
    res->frame = NULL;

    res->cc = avcodec_alloc_context3(decoder);
    if (res->cc == NULL) {
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

Decoder* ffw_decoder_from_codec_parameters(const AVCodecParameters* params) {
    const AVCodec* decoder = avcodec_find_decoder(params->codec_id);
    if (decoder == NULL) {
        return NULL;
    }

    Decoder* res = malloc(sizeof(Decoder));
    if (res == NULL) {
        return NULL;
    }

    res->decoder = decoder;
    res->options = NULL;
    res->cc = NULL;
    res->frame = NULL;

    res->cc = avcodec_alloc_context3(decoder);
    if (res->cc == NULL) {
        goto err;
    }

    res->frame = av_frame_alloc();
    if (res->frame == NULL) {
        goto err;
    }

    if (avcodec_parameters_to_context(res->cc, params) < 0) {
        goto err;
    }

    return res;

err:
    ffw_decoder_free(res);

    return NULL;
}

int ffw_decoder_set_extradata(Decoder* decoder, const uint8_t* extradata, int size) {
    if (decoder->cc->extradata) {
        av_freep(&decoder->cc->extradata);
    }

    decoder->cc->extradata_size = 0;

    if (extradata == NULL || size <= 0) {
        decoder->cc->extradata = NULL;
    } else {
        decoder->cc->extradata = av_mallocz(size + AV_INPUT_BUFFER_PADDING_SIZE);
        if (decoder->cc->extradata == NULL) {
            return -1;
        }

        memcpy(decoder->cc->extradata, extradata, size);

        decoder->cc->extradata_size = size;
    }

    return 0;
}

int ffw_decoder_set_initial_option(Decoder* decoder, const char* key, const char* value) {
    return av_dict_set(&decoder->options, key, value, 0);
}

void ffw_decoder_set_pkt_timebase(Decoder* decoder, int num, int den) {
    AVRational r;

    r.num = num;
    r.den = den;

    decoder->cc->pkt_timebase = r;
}

int ffw_decoder_open(Decoder* decoder) {
    return avcodec_open2(decoder->cc, decoder->decoder, &decoder->options);
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

AVCodecParameters* ffw_decoder_get_codec_parameters(const Decoder* decoder) {
    AVCodecParameters* params;
    int ret;

    params = avcodec_parameters_alloc();
    if (params == NULL) {
        return NULL;
    }

    ret = avcodec_parameters_from_context(params, decoder->cc);
    if (ret < 0) {
        goto err;
    }

    return params;

err:
    avcodec_parameters_free(&params);

    return NULL;
}

void ffw_decoder_free(Decoder* decoder) {
    if (decoder == NULL) {
        return;
    }

    av_frame_free(&decoder->frame);

    if (decoder->cc->extradata != NULL) {
        av_free(decoder->cc->extradata);

        decoder->cc->extradata_size = 0;
        decoder->cc->extradata = NULL;
    }

    avcodec_free_context(&decoder->cc);
    av_dict_free(&decoder->options);
    free(decoder);
}

typedef struct Encoder {
    struct AVDictionary* options;
    struct AVCodecContext* cc;
    const struct AVCodec* codec;
    struct AVPacket* packet;
} Encoder;

Encoder* ffw_encoder_new(const char* codec);
Encoder* ffw_encoder_from_codec_parameters(const AVCodecParameters* params);
int ffw_encoder_get_pixel_format(const Encoder* encoder);
int ffw_encoder_get_width(const Encoder* encoder);
int ffw_encoder_get_height(const Encoder* encoder);
int ffw_encoder_get_sample_format(const Encoder* encoder);
int ffw_encoder_get_sample_rate(const Encoder* encoder);
void ffw_encoder_set_time_base(Encoder* encoder, int num, int den);
void ffw_encoder_set_bit_rate(Encoder* encoder, int64_t bit_rate);
void ffw_encoder_set_pixel_format(Encoder* encoder, int format);
void ffw_encoder_set_width(Encoder* encoder, int width);
void ffw_encoder_set_height(Encoder* encoder, int height);
void ffw_encoder_set_sample_format(Encoder* encoder, int format);
void ffw_encoder_set_sample_rate(Encoder* encoder, int sample_rate);
int ffw_encoder_set_initial_option(Encoder* encoder, const char* key, const char* value);
int ffw_encoder_open(Encoder* encoder);
int ffw_encoder_push_frame(Encoder* encoder, const AVFrame* frame);
int ffw_encoder_take_packet(Encoder* encoder, AVPacket** packet);
void ffw_encoder_free(Encoder* encoder);

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
const AVChannelLayout * ffw_encoder_get_channel_layout(const Encoder* encoder);
int ffw_encoder_set_channel_layout(Encoder* encoder, const AVChannelLayout* layout);
#else
const uint64_t * ffw_encoder_get_channel_layout(const Encoder* encoder);
int ffw_encoder_set_channel_layout(Encoder* encoder, const uint64_t* layout);
#endif

Encoder* ffw_encoder_new(const char* codec) {
    const AVCodec* encoder = avcodec_find_encoder_by_name(codec);
    if (encoder == NULL) {
        return NULL;
    }

    Encoder* res = malloc(sizeof(Encoder));
    if (res == NULL) {
        return NULL;
    }

    res->codec = encoder;
    res->options = NULL;
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

Encoder* ffw_encoder_from_codec_parameters(const AVCodecParameters* params) {
    const AVCodec* encoder = avcodec_find_encoder(params->codec_id);
    if (encoder == NULL) {
        return NULL;
    }

    Encoder* res = malloc(sizeof(Encoder));
    if (res == NULL) {
        return NULL;
    }

    res->codec = encoder;
    res->options = NULL;
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

    if (avcodec_parameters_to_context(res->cc, params) < 0) {
        goto err;
    }

    return res;

err:
    ffw_encoder_free(res);

    return NULL;
}

AVCodecParameters* ffw_encoder_get_codec_parameters(const Encoder* encoder) {
    AVCodecParameters* params;
    int ret;

    params = avcodec_parameters_alloc();
    if (params == NULL) {
        return NULL;
    }

    ret = avcodec_parameters_from_context(params, encoder->cc);
    if (ret < 0) {
        goto err;
    }

    return params;

err:
    avcodec_parameters_free(&params);

    return NULL;
}

int ffw_encoder_get_pixel_format(const Encoder* encoder) {
    return encoder->cc->pix_fmt;
}

int ffw_encoder_get_width(const Encoder* encoder) {
    return encoder->cc->width;
}

int ffw_encoder_get_height(const Encoder* encoder) {
    return encoder->cc->height;
}


int ffw_encoder_get_sample_format(const Encoder* encoder) {
    return encoder->cc->sample_fmt;
}

int ffw_encoder_get_sample_rate(const Encoder* encoder) {
    return encoder->cc->sample_rate;
}

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
const AVChannelLayout * ffw_encoder_get_channel_layout(const Encoder* encoder) {
    return &encoder->cc->ch_layout;
}
#else
const uint64_t * ffw_encoder_get_channel_layout(const Encoder* encoder) {
    return &encoder->cc->channel_layout;
}
#endif

int ffw_encoder_get_frame_size(const Encoder* encoder) {
    return encoder->cc->frame_size;
}

void ffw_encoder_set_time_base(Encoder* encoder, int num, int den) {
    encoder->cc->time_base.num = num;
    encoder->cc->time_base.den = den;
}

void ffw_encoder_set_bit_rate(Encoder* encoder, int64_t bit_rate) {
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

void ffw_encoder_set_sample_format(Encoder* encoder, int format) {
    encoder->cc->sample_fmt = format;
}

void ffw_encoder_set_sample_rate(Encoder* encoder, int sample_rate) {
    encoder->cc->sample_rate = sample_rate;
}

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
int ffw_encoder_set_channel_layout(Encoder* encoder, const AVChannelLayout* layout) {
    return av_channel_layout_copy(&encoder->cc->ch_layout, layout);
}
#else
int ffw_encoder_set_channel_layout(Encoder* encoder, const uint64_t* layout) {
    uint64_t ch_layout = *layout;

    encoder->cc->channel_layout = ch_layout;
    encoder->cc->channels = av_get_channel_layout_nb_channels(ch_layout);

    return 0;
}
#endif

int ffw_encoder_set_initial_option(Encoder* encoder, const char* key, const char* value) {
    return av_dict_set(&encoder->options, key, value, 0);
}

int ffw_encoder_open(Encoder* encoder) {
    return avcodec_open2(encoder->cc, encoder->codec, &encoder->options);
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
    av_dict_free(&encoder->options);
    free(encoder);
}
