#include <libavformat/avformat.h>

void ffw_stream_get_time_base(const AVStream* stream, int* num, int* den) {
    *num = stream->time_base.num;
    *den = stream->time_base.den;
}

void ffw_stream_set_time_base(AVStream* stream, int num, int den) {
    stream->time_base.num = num;
    stream->time_base.den = den;
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

int ffw_stream_get_id(const AVStream* stream) {
    return stream->id;
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

void ffw_stream_set_id(AVStream* stream, int id) {
    stream->id = id;
}

#ifdef FFW_FEATURE_STREAM_SIDE_DATA

size_t ffw_stream_get_nb_side_data(const AVStream* stream) {
    return stream->nb_side_data;
}

const AVPacketSideData* ffw_stream_get_side_data(const AVStream* stream, size_t index) {
    return &stream->side_data[index];
}

int ffw_stream_add_side_data(
    AVStream* stream,
    enum AVPacketSideDataType data_type,
    const uint8_t* data,
    size_t size)
{
    void* dup_data;
    int ret;

    if (!(dup_data = av_memdup(data, size))) {
        return AVERROR(ENOMEM);
    }

    if ((ret = av_stream_add_side_data(stream, data_type, dup_data, size)) < 0) {
        av_free(dup_data);
    }

    return ret;
}

#endif // FFW_FEATURE_STREAM_SIDE_DATA
