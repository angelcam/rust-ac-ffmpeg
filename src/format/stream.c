#include <libavformat/avformat.h>

void ffw_stream_get_time_base(const AVStream* stream, uint32_t* num, uint32_t* den);
int64_t ffw_stream_get_start_time(const AVStream* stream);
int64_t ffw_stream_get_duration(const AVStream* stream);
int64_t ffw_stream_get_nb_frames(const AVStream* stream);
AVCodecParameters* ffw_stream_get_codec_parameters(const AVStream* stream);
int ffw_stream_set_metadata(AVStream* stream, const char* key, const char* value);

void ffw_stream_get_time_base(const AVStream* stream, uint32_t* num, uint32_t* den) {
    *num = stream->time_base.num;
    *den = stream->time_base.den;
}

int64_t ffw_stream_get_start_time(const AVStream* stream) {
    return stream->start_time;
}

int64_t ffw_stream_get_duration(const AVStream* stream) {
    return stream->duration;
}

int64_t ffw_stream_get_nb_frames(const AVStream* stream) {
    return stream->nb_frames;
}

AVCodecParameters* ffw_stream_get_codec_parameters(const AVStream* stream) {
    AVCodecParameters* res = avcodec_parameters_alloc();
    if (!res) {
        return NULL;
    }

    if (avcodec_parameters_copy(res, stream->codecpar) < 0) {
        goto err;
    }

    return res;

err:
    avcodec_parameters_free(&res);

    return NULL;
}

int ffw_stream_set_metadata(AVStream* stream, const char* key, const char* value) {
    return av_dict_set(&stream->metadata, key, value, 0);
}
