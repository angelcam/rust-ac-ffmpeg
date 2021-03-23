#include <libavformat/avformat.h>
#include <libavutil/opt.h>

AVInputFormat* ffw_find_input_format(const char* short_name) {
    return av_find_input_format(short_name);
}

typedef struct Demuxer {
    AVFormatContext* fc;
    AVDictionary* options;
} Demuxer;

Demuxer* ffw_demuxer_new();
int ffw_demuxer_init(Demuxer* demuxer, AVIOContext* io_context, AVInputFormat* format);
int ffw_demuxer_set_initial_option(Demuxer* demuxer, const char* key, const char* value);
int ffw_demuxer_set_option(Demuxer* demuxer, const char* key, const char* value);
int ffw_demuxer_find_stream_info(Demuxer* demuxer, int64_t max_analyze_duration);
unsigned ffw_demuxer_get_nb_streams(const Demuxer* demuxer);
AVCodecParameters* ffw_demuxer_get_codec_parameters(const Demuxer* demuxer, unsigned stream_index);
int ffw_demuxer_read_frame(Demuxer* demuxer, AVPacket** packet, uint32_t* tb_num, uint32_t* tb_den);
AVFormatContext* ffw_demuxer_get_format_context(Demuxer* demuxer);
void ffw_demuxer_free(Demuxer* demuxer);

Demuxer* ffw_demuxer_new() {
    Demuxer* demuxer = calloc(1, sizeof(Demuxer));
    if (!demuxer) {
        return NULL;
    }

    demuxer->fc = avformat_alloc_context();
    if (!demuxer->fc) {
        goto err;
    }

    return demuxer;

err:
    ffw_demuxer_free(demuxer);

    return NULL;
}

int ffw_demuxer_init(Demuxer* demuxer, AVIOContext* avio_context, AVInputFormat* format) {
    int ret;

    demuxer->fc->pb = avio_context;

    ret = avformat_open_input(&demuxer->fc, NULL, format, &demuxer->options);
    if (ret < 0) {
        return ret;
    }

    av_dict_free(&demuxer->options);

    return ret;
}

int ffw_demuxer_set_initial_option(Demuxer* demuxer, const char* key, const char* value) {
    return av_dict_set(&demuxer->options, key, value, 0);
}

int ffw_demuxer_set_option(Demuxer* demuxer, const char* key, const char* value) {
    return av_opt_set(demuxer->fc, key, value, AV_OPT_SEARCH_CHILDREN);
}

int ffw_demuxer_find_stream_info(Demuxer* demuxer, int64_t max_analyze_duration) {
    AVRational micro;
    AVRational dst;

    micro.num = 1;
    micro.den = 1000000;

    dst.num = 1;
    dst.den = AV_TIME_BASE;

    max_analyze_duration = av_rescale_q(max_analyze_duration, micro, dst);

    demuxer->fc->max_analyze_duration = max_analyze_duration;

    return avformat_find_stream_info(demuxer->fc, NULL);
}

unsigned ffw_demuxer_get_nb_streams(const Demuxer* demuxer) {
    return demuxer->fc->nb_streams;
}

AVCodecParameters* ffw_demuxer_get_codec_parameters(const Demuxer* demuxer, unsigned stream_index) {
    AVCodecParameters* res;
    const AVStream* s;

    s = demuxer->fc->streams[stream_index];

    res = avcodec_parameters_alloc();
    if (!res) {
        return NULL;
    }

    if (avcodec_parameters_copy(res, s->codecpar) < 0) {
        goto err;
    }

    return res;

err:
    avcodec_parameters_free(&res);

    return NULL;
}

int ffw_demuxer_read_frame(Demuxer* demuxer, AVPacket** packet, uint32_t* tb_num, uint32_t* tb_den) {
    AVStream* stream;
    AVPacket* res;
    AVPacket tmp;
    int ret;

    av_init_packet(&tmp);

    ret = av_read_frame(demuxer->fc, &tmp);
    if (ret == AVERROR_EOF) {
        *packet = NULL;
        return 0;
    } else if (ret < 0) {
        return ret;
    }

    res = av_packet_clone(&tmp);

    av_packet_unref(&tmp);

    if (!res) {
        return AVERROR(ENOMEM);
    }

    stream = demuxer->fc->streams[res->stream_index];

    *packet = res;

    *tb_num = stream->time_base.num;
    *tb_den = stream->time_base.den;

    return ret;
}

AVFormatContext* ffw_demuxer_get_format_context(Demuxer* demuxer) {
    return demuxer->fc;
}

void ffw_demuxer_free(Demuxer* demuxer) {
    if (!demuxer) {
        return;
    }

    avformat_close_input(&demuxer->fc);
    av_dict_free(&demuxer->options);

    free(demuxer);
}
