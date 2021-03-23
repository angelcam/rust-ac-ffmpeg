#include <libavformat/avformat.h>
#include <libavutil/opt.h>

enum SeekType {
    Time,
    Byte,
    Frame,
};

enum SeekTarget {
    From,
    UpTo,
    Precise,
};

typedef struct Stream {
  AVFormatContext* fc;
  AVStream *stream;
} Stream;

Stream* ffw_stream_from_format_context(AVFormatContext* context, unsigned stream_index);
int ffw_stream_get_index(const Stream* stream);
int ffw_stream_get_id(const Stream* stream);
void ffw_stream_get_time_base(const Stream* stream, uint32_t* num, uint32_t* den);
int64_t ffw_stream_get_start_time(const Stream* stream);
int64_t ffw_stream_get_duration(const Stream* stream);
int64_t ffw_stream_get_nb_frames(const Stream* stream);
int ffw_stream_seek_frame(Stream* stream, unsigned stream_index, int64_t timestamp, int seek_by, int direction);
void ffw_stream_free(Stream* stream);

Stream* ffw_stream_from_format_context(AVFormatContext* context, unsigned stream_index) {
    Stream* res = (Stream*)malloc(sizeof(Stream));

    res->fc = context;
    res->stream = context->streams[stream_index];

    return res;
}

int ffw_stream_get_index(const Stream* stream) {
    return stream->stream->index;
}

int ffw_stream_get_id(const Stream* stream) {
    return stream->stream->id;
}

void ffw_stream_get_time_base(const Stream* stream, uint32_t* num, uint32_t* den) {
    *num = stream->stream->time_base.num;
    *den = stream->stream->time_base.den;
}

int64_t ffw_stream_get_start_time(const Stream* stream) {
    return stream->stream->start_time;
}

int64_t ffw_stream_get_duration(const Stream* stream) {
    return stream->stream->duration;
}

// FIXME: Math is off
int64_t ffw_stream_get_nb_frames(const Stream* stream) {
    AVRational avg_frame_rate;
    double avg_frame_rate_double;
    int64_t duration;

    avg_frame_rate = stream->stream->avg_frame_rate;
    avg_frame_rate_double = (double)avg_frame_rate.num / avg_frame_rate.den;
    duration = stream->stream->duration;

    return duration * avg_frame_rate_double;
}

int ffw_stream_seek_frame(Stream* stream, unsigned stream_index, int64_t timestamp, int seek_by, int seek_target) {
  int flags;

  flags = 0;

  if (seek_by == Byte) { flags |= AVSEEK_FLAG_BYTE; }
  else if (seek_by == Frame) { flags |= AVSEEK_FLAG_FRAME; }

  if (seek_target == UpTo) { flags |= AVSEEK_FLAG_BACKWARD; }
  else if (seek_target == Precise) { flags |= AVSEEK_FLAG_ANY; }

  return av_seek_frame(stream->fc, (int)stream_index, timestamp, flags);
}

void ffw_stream_free(Stream* stream) {
  free(stream);
}
