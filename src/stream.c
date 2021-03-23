#include <libavformat/avformat.h>
#include <libavutil/opt.h>

enum SeekType {
    Time,
    Byte,
    Frame,
};

enum Direction {
    Forward,
    Backward,
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
int ffw_stream_seek_frame(Stream* stream, unsigned stream_index, int64_t timestamp, int seek_by, int direction, int seek_to_keyframes_only);
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

int64_t ffw_stream_get_nb_frames(const Stream* stream) {
    return stream->stream->nb_frames;
}

int ffw_stream_seek_frame(Stream* stream, unsigned stream_index, int64_t timestamp, int seek_by, int direction, int seek_to_keyframes_only) {
  int flags = 0;

  if (seek_by == Byte) { flags |= AVSEEK_FLAG_BYTE; }
  else if (seek_by == Frame) { flags |= AVSEEK_FLAG_FRAME; }

  if (direction == Backward) { flags |= AVSEEK_FLAG_BACKWARD; }

  if (seek_to_keyframes_only == 0) { flags |= AVSEEK_FLAG_ANY; }

  return av_seek_frame(stream->fc, (int)stream_index, timestamp, flags);
}

void ffw_stream_free(Stream* stream) {
  free(stream);
}

