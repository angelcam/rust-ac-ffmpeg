#include <libavformat/avformat.h>
#include <libavutil/opt.h>
#include <libavutil/avstring.h>

#define SEEK_TYPE_TIME  0
#define SEEK_TYPE_BYTE  1
#define SEEK_TYPE_FRAME 2

#define SEEK_TARGET_FROM    0
#define SEEK_TARGET_UP_TO   1
#define SEEK_TARGET_PRECISE 2

AVInputFormat* ffw_guess_input_format(
    const char* short_name,
    const char* file_name,
    const char* mime_type) {
    const AVInputFormat* fmt = NULL;
    const AVInputFormat* res = NULL;
    void* i = NULL;
    int score;
    int max_score = 0;

    while ((fmt = av_demuxer_iterate(&i))) {
        score = 0;

        if (short_name && fmt->name && av_match_name(short_name, fmt->name)) {
            score += 100;
        }

        if (mime_type && fmt->mime_type && av_match_name(mime_type, fmt->mime_type)) {
            score += 10;
        }

        if (file_name && fmt->extensions && av_match_ext(file_name, fmt->extensions)) {
            score += 5;
        }

        if (score > max_score) {
            max_score = score;
            res = fmt;
        }
    }

    return (AVInputFormat*)res;
}

const char* ffw_input_format_name(
    AVInputFormat* input_format
) {
    return input_format->name;
}

typedef struct Demuxer {
    AVFormatContext* fc;
    AVDictionary* options;
    AVPacket* packet;
} Demuxer;

Demuxer* ffw_demuxer_new();
int ffw_demuxer_init(Demuxer* demuxer, AVIOContext* io_context, AVInputFormat* format);
int ffw_demuxer_set_initial_option(Demuxer* demuxer, const char* key, const char* value);
int ffw_demuxer_set_option(Demuxer* demuxer, const char* key, const char* value);
int ffw_demuxer_find_stream_info(Demuxer* demuxer, int64_t max_analyze_duration);
unsigned ffw_demuxer_get_nb_streams(const Demuxer* demuxer);
AVStream* ffw_demuxer_get_stream(Demuxer* demuxer, unsigned stream_index);
const struct AVInputFormat* ffw_demuxer_get_input_format(Demuxer* demuxer);
int ffw_demuxer_read_frame(Demuxer* demuxer, AVPacket** packet, uint32_t* tb_num, uint32_t* tb_den);
int ffw_demuxer_seek(Demuxer* demuxer, int64_t timestamp, int seek_by, int seek_target);
void ffw_demuxer_free(Demuxer* demuxer);

Demuxer* ffw_demuxer_new() {
    Demuxer* demuxer = calloc(1, sizeof(Demuxer));
    if (!demuxer) {
        return NULL;
    }

    demuxer->packet = av_packet_alloc();
    if (!demuxer->packet) {
        goto err;
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

AVStream* ffw_demuxer_get_stream(Demuxer* demuxer, unsigned stream_index) {
    return demuxer->fc->streams[stream_index];
}

const struct AVInputFormat* ffw_demuxer_get_input_format(Demuxer* demuxer) {
    return demuxer->fc->iformat;
}

int ffw_demuxer_read_frame(Demuxer* demuxer, AVPacket** packet, uint32_t* tb_num, uint32_t* tb_den) {
    AVStream* stream;
    AVPacket* res;
    int ret;

    ret = av_read_frame(demuxer->fc, demuxer->packet);
    if (ret == AVERROR_EOF) {
        av_packet_unref(demuxer->packet);
        *packet = NULL;
        return 0;
    } else if (ret < 0) {
        av_packet_unref(demuxer->packet);
        return ret;
    }

    // NOTE: Older versions of FFmpeg can guarantee packet buffer lifetime only
    // until the next call of av_read_frame().
    res = av_packet_clone(demuxer->packet);

    av_packet_unref(demuxer->packet);

    if (!res) {
        return AVERROR(ENOMEM);
    }

    stream = demuxer->fc->streams[res->stream_index];

    *packet = res;

    *tb_num = stream->time_base.num;
    *tb_den = stream->time_base.den;

    return ret;
}

int ffw_demuxer_seek(Demuxer* demuxer, int64_t timestamp, int seek_by, int seek_target) {
    int flags;

    flags = 0;

    switch (seek_by) {
        case SEEK_TYPE_BYTE:
            flags |= AVSEEK_FLAG_BYTE;
            break;
        case SEEK_TYPE_FRAME:
            flags |= AVSEEK_FLAG_FRAME;
            break;
        default:
            break;
    }

    switch (seek_target) {
        case SEEK_TARGET_UP_TO:
            flags |= AVSEEK_FLAG_BACKWARD;
            break;
        case SEEK_TARGET_PRECISE:
            flags |= AVSEEK_FLAG_ANY;
            break;
        default:
            break;
    }

    return av_seek_frame(demuxer->fc, -1, timestamp, flags);
}

void ffw_demuxer_free(Demuxer* demuxer) {
    if (!demuxer) {
        return;
    }

    av_packet_free(&demuxer->packet);
    avformat_close_input(&demuxer->fc);
    av_dict_free(&demuxer->options);

    free(demuxer);
}
