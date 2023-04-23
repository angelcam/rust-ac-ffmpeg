#include <libavcodec/avcodec.h>

typedef struct SubtitleTranscoder {
    const struct AVCodec* decoder;
    struct AVCodecContext* decoder_ctx;
    struct AVDictionary* decoder_options;
    const struct AVCodec* encoder;
    struct AVCodecContext* encoder_ctx;
    struct AVDictionary* encoder_options;
    struct AVSubtitle* subtitle;
} SubtitleTranscoder;

SubtitleTranscoder* ffw_subtitle_transcoder_new(const char* in_codec, const char* out_codec);
SubtitleTranscoder* ffw_subtitle_transcoder_from_codec_parameters(const AVCodecParameters* in_params, const AVCodecParameters* out_params);
int ffw_subtitle_decoder_set_initial_option(SubtitleTranscoder* transcoder, const char* key, const char* value);
int ffw_subtitle_encoder_set_initial_option(SubtitleTranscoder* transcoder, const char* key, const char* value);
int ffw_subtitle_transcoder_open(SubtitleTranscoder* transcoder, int in_tb_num, int in_tb_den, int out_tb_num, int out_tb_den);
int ffw_subtitle_transcoder_push_packet(SubtitleTranscoder* transcoder, AVPacket* packet);
int ffw_subtitle_transcoder_take_packet(SubtitleTranscoder* transcoder, AVPacket** packet);
void ffw_subtitle_transcoder_free(SubtitleTranscoder* transcoder);

SubtitleTranscoder* ffw_subtitle_transcoder_new(const char* in_codec, const char* out_codec) {
    const AVCodec* decoder = avcodec_find_decoder_by_name(in_codec);
    if (decoder == NULL) {
        return NULL;
    }

    const AVCodec* encoder = avcodec_find_encoder_by_name(out_codec);
    if (encoder == NULL) {
        return NULL;
    }

    SubtitleTranscoder* res = malloc(sizeof(SubtitleTranscoder));
    if (res == NULL) {
        return NULL;
    }

    res->decoder = decoder;
    res->decoder_ctx = avcodec_alloc_context3(decoder);
    if (res->decoder_ctx == NULL) {
        goto err;
    }
    res->decoder_options = NULL;

    res->encoder = encoder;
    res->encoder_ctx = avcodec_alloc_context3(encoder);
    if (res->encoder == NULL) {
        goto err;
    }
    res->encoder_options = NULL;

    res->subtitle = malloc(sizeof(AVSubtitle));
    return res;

err:
    ffw_subtitle_transcoder_free(res);

    return NULL;
}

SubtitleTranscoder* ffw_subtitle_transcoder_from_codec_parameters(const AVCodecParameters* in_params, const AVCodecParameters* out_params) {

    const AVCodec* decoder = avcodec_find_decoder(in_params->codec_id);
    if (decoder == NULL) {
        return NULL;
    }

    const AVCodec* encoder = avcodec_find_encoder(out_params->codec_id);
    if (encoder == NULL) {
        return NULL;
    }

    SubtitleTranscoder* res = malloc(sizeof(SubtitleTranscoder));
    if (res == NULL) {
        return NULL;
    }

    res->decoder = decoder;
    res->decoder_ctx = avcodec_alloc_context3(decoder);
    if (res->decoder_ctx == NULL) {
        goto err;
    }
    if (avcodec_parameters_to_context(res->decoder_ctx, in_params) < 0) {
        goto err;
    }
    res->decoder_options = NULL;

    res->encoder = encoder;
    res->encoder_ctx = avcodec_alloc_context3(encoder);
    if (res->encoder == NULL) {
        goto err;
    }
    if (avcodec_parameters_to_context(res->encoder_ctx, out_params) < 0) {
        goto err;
    }
    res->encoder_options = NULL;

    res->subtitle = malloc(sizeof(AVSubtitle));
    return res;

err:
    ffw_subtitle_transcoder_free(res);

    return NULL;
}

int ffw_subtitle_decoder_set_initial_option(SubtitleTranscoder* transcoder, const char* key, const char* value) {
    return av_dict_set(&transcoder->decoder_options, key, value, 0);
}

int ffw_subtitle_encoder_set_initial_option(SubtitleTranscoder* transcoder, const char* key, const char* value) {
    return av_dict_set(&transcoder->encoder_options, key, value, 0);
}

int ffw_subtitle_transcoder_open(SubtitleTranscoder* transcoder, int in_tb_num, int in_tb_den, int out_tb_num, int out_tb_den) {
    // open decoder
    AVRational in_tb;
    in_tb.num = in_tb_num;
    in_tb.den = in_tb_den;
    transcoder->decoder_ctx->time_base = in_tb;
    transcoder->decoder_ctx->pkt_timebase = in_tb;
    int ret = avcodec_open2(transcoder->decoder_ctx, transcoder->decoder, &transcoder->decoder_options);
    if (ret != 0) {
        return ret;
    }

    // pass through e.g style headers from decoder
    transcoder->encoder_ctx->subtitle_header = transcoder->decoder_ctx->subtitle_header;
    transcoder->encoder_ctx->subtitle_header_size = transcoder->decoder_ctx->subtitle_header_size;

    // open encoder
    AVRational out_tb;
    out_tb.num = out_tb_num;
    out_tb.den = out_tb_den;
    transcoder->encoder_ctx->time_base = out_tb;
    transcoder->encoder_ctx->pkt_timebase = out_tb;
    ret = avcodec_open2(transcoder->encoder_ctx, transcoder->encoder, &transcoder->encoder_options);
    return ret;
}

int ffw_subtitle_transcoder_push_packet(SubtitleTranscoder* transcoder, AVPacket* packet) {
    int got_output;
    int ret = avcodec_decode_subtitle2(transcoder->decoder_ctx, transcoder->subtitle, &got_output, packet);
    if (ret < 0) {
        av_log(NULL, AV_LOG_ERROR, "failed to decode subtitle packet\n");
        return ret;
    }
    if (got_output == 0) {
        return AVERROR(EAGAIN);
    }
    return 0;
}

int ffw_subtitle_transcoder_take_packet(SubtitleTranscoder* transcoder, AVPacket** packet) {
    if (transcoder->subtitle->num_rects == 0) {
        return AVERROR(EAGAIN);
    }

    int subtitle_out_max_size = 1024 * 1024; // these are the values ffmpeg uses
    int subtitle_out_size;
    static uint8_t* subtitle_out;
    subtitle_out = av_malloc(subtitle_out_max_size);
    if (!subtitle_out) {
        av_log(NULL, AV_LOG_FATAL, "Failed to allocate subtitle out\n");
        return 1;
    }

    subtitle_out_size = avcodec_encode_subtitle(transcoder->encoder_ctx, subtitle_out, subtitle_out_max_size, transcoder->subtitle);
    if (subtitle_out_size < 0) {
        av_log(NULL, AV_LOG_FATAL, "Subtitle encoding failed\n");
        return 1;
    }

    AVPacket* out = av_packet_alloc();
    out->data = subtitle_out;
    out->size = subtitle_out_size;
    out->pts = transcoder->subtitle->pts;
    out->duration = transcoder->subtitle->end_display_time;
    *packet = out;

    avsubtitle_free(transcoder->subtitle);
    return 0;
}

void ffw_subtitle_transcoder_free(SubtitleTranscoder* transcoder)
{
    if (transcoder == NULL) {
        return;
    }

    if (transcoder->subtitle != NULL) {
        avsubtitle_free(transcoder->subtitle);
    }

    if (transcoder->decoder_ctx != NULL) {
        avcodec_free_context(&transcoder->decoder_ctx);
    }
    if (transcoder->decoder_options != NULL) {
        avcodec_free_context(&transcoder->decoder_ctx);
    }
    if (transcoder->encoder_ctx != NULL) {
        avcodec_free_context(&transcoder->encoder_ctx);
    }
    if (transcoder->encoder_options != NULL) {
        avcodec_free_context(&transcoder->decoder_ctx);
    }
    free(transcoder);
}
