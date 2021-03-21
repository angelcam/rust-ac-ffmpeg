#include <libavformat/avformat.h>
#include <libavutil/opt.h>

int ffw_stream_get_index(const AVStream* stream);
int ffw_stream_get_id(const AVStream* stream);
void ffw_stream_get_time_base(const AVStream* stream, uint32_t* num, uint32_t* den);
int64_t ffw_stream_get_start_time(const AVStream* stream);
int64_t ffw_stream_get_duration(const AVStream* stream);
int64_t ffw_stream_get_nb_frames(const AVStream* stream);

int ffw_stream_get_index(const AVStream* stream) {
    return stream->index;
}

int ffw_stream_get_id(const AVStream* stream) {
    return stream->id;
}

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
