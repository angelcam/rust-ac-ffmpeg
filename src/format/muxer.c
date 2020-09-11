#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>

#include <stdlib.h>

AVOutputFormat* ffw_guess_output_format(
    const char* short_name,
    const char* file_name,
    const char* mime_type) {
    return av_guess_format(short_name, file_name, mime_type);
}

typedef struct Muxer {
    AVFormatContext* fc;
    AVDictionary* options;
    int initialized;
} Muxer;

Muxer* ffw_muxer_new();
unsigned ffw_muxer_get_nb_streams(const Muxer*);
int ffw_muxer_new_stream(Muxer*, const AVCodecParameters*);
int ffw_muxer_init(Muxer*, AVIOContext*, AVOutputFormat*);
int ffw_muxer_get_option(Muxer*, const char*, uint8_t**);
int ffw_muxer_set_initial_option(Muxer*, const char*, const char*);
int ffw_muxer_set_option(Muxer*, const char*, const char*);
int ffw_muxer_write_frame(Muxer*, AVPacket*, uint32_t, uint32_t);
int ffw_muxer_interleaved_write_frame(Muxer*, AVPacket*, uint32_t, uint32_t);
void ffw_muxer_free(Muxer*);

Muxer* ffw_muxer_new() {
    Muxer* muxer = malloc(sizeof(Muxer));
    if (muxer == NULL) {
        return NULL;
    }

    muxer->fc = NULL;
    muxer->options = NULL;
    muxer->initialized = 0;

    muxer->fc = avformat_alloc_context();
    if (muxer->fc == NULL) {
        goto err;
    }

    return muxer;

err:
    ffw_muxer_free(muxer);

    return NULL;
}

unsigned ffw_muxer_get_nb_streams(const Muxer* muxer) {
    return muxer->fc->nb_streams;
}

int ffw_muxer_new_stream(Muxer* muxer, const AVCodecParameters* params) {
    AVStream* s;
    int ret;

    s = avformat_new_stream(muxer->fc, NULL);
    if (s == NULL) {
        return AVERROR(ENOMEM);
    }

    ret = avcodec_parameters_copy(s->codecpar, params);
    if (ret < 0) {
        return ret;
    }

    s->codecpar->codec_tag = 0;

    return s->index;
}

int ffw_muxer_init(
    Muxer* muxer,
    AVIOContext* avio_context,
    AVOutputFormat* format) {
    int ret;

    muxer->fc->pb = avio_context;
    muxer->fc->oformat = format;

    ret = avformat_write_header(muxer->fc, &muxer->options);
    if (ret < 0) {
        return ret;
    }

    muxer->initialized = 1;

    av_dict_free(&muxer->options);

    return ret;
}

int ffw_muxer_set_initial_option(Muxer* muxer, const char* key, const char* value) {
    return av_dict_set(&muxer->options, key, value, 0);
}

int ffw_muxer_set_option(Muxer* muxer, const char* key, const char* value) {
    return av_opt_set(muxer->fc, key, value, AV_OPT_SEARCH_CHILDREN);
}

static int ffw_rescale_packet_timestamps(Muxer* muxer, AVPacket* packet, uint32_t src_tb_num, uint32_t src_tb_den) {
    AVStream* stream;
    AVRational src_tb;

    unsigned stream_index;

    if (packet == NULL) {
        return 0;
    }

    stream_index = packet->stream_index;

    if (stream_index > muxer->fc->nb_streams) {
        return AVERROR(EINVAL);
    }

    stream = muxer->fc->streams[stream_index];

    src_tb.num = src_tb_num;
    src_tb.den = src_tb_den;

    av_packet_rescale_ts(packet, src_tb, stream->time_base);

    return 0;
}

int ffw_muxer_write_frame(Muxer* muxer, AVPacket* packet, uint32_t tb_num, uint32_t tb_den) {
    int ret = ffw_rescale_packet_timestamps(muxer, packet, tb_num, tb_den);

    if (ret < 0) {
        return ret;
    }

    return av_write_frame(muxer->fc, packet);
}

int ffw_muxer_interleaved_write_frame(Muxer* muxer, AVPacket* packet, uint32_t tb_num, uint32_t tb_den) {
    int ret = ffw_rescale_packet_timestamps(muxer, packet, tb_num, tb_den);

    if (ret < 0) {
        return ret;
    }

    return av_interleaved_write_frame(muxer->fc, packet);
}

void ffw_muxer_free(Muxer* muxer) {
    if (muxer == NULL) {
        return;
    }

    if (muxer->initialized) {
        av_write_trailer(muxer->fc);
    }

    avformat_free_context(muxer->fc);
    av_dict_free(&muxer->options);

    free(muxer);
}
