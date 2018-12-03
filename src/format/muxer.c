#include<libavformat/avformat.h>
#include<libavformat/avio.h>
#include<libavcodec/avcodec.h>

#include<stdlib.h>

AVOutputFormat* ffw_guess_output_format(
    const char* short_name,
    const char* file_name,
    const char* mime_type) {
    return av_guess_format(short_name, file_name, mime_type);
}

typedef struct Muxer {
    AVFormatContext* fc;
    int initialized;
} Muxer;

Muxer* ffw_muxer_new();
unsigned ffw_muxer_get_nb_streams(const Muxer*);
int ffw_muxer_new_stream(Muxer*, const char*);
void ffw_muxer_set_stream_extradata(Muxer*, unsigned, uint8_t*, int);
int ffw_muxer_init(Muxer*, AVIOContext*, AVOutputFormat*, AVDictionary**);
int ffw_muxer_write_frame(Muxer*, AVPacket*);
int ffw_muxer_interleaved_write_frame(Muxer*, AVPacket*);
void ffw_muxer_free(Muxer*);

Muxer* ffw_muxer_new() {
    Muxer* muxer = malloc(sizeof(Muxer));
    if (muxer == NULL) {
        return NULL;
    }

    muxer->fc = NULL;
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

int ffw_muxer_new_stream(Muxer* muxer, const char* codec) {
    AVCodec* encoder = avcodec_find_encoder_by_name(codec);
    if (encoder == NULL) {
        return -1;
    }

    AVStream* s = avformat_new_stream(muxer->fc, encoder);
    if (s == NULL) {
        return -1;
    }

    return s->index;
}

void ffw_muxer_set_stream_extradata(
    Muxer* muxer,
    unsigned stream_index,
    uint8_t* extradata,
    int extradata_size) {
    if (stream_index >= muxer->fc->nb_streams) {
        return;
    }

    AVStream* stream = muxer->fc->streams[stream_index];

    stream->codecpar->extradata = extradata;
    stream->codecpar->extradata_size = extradata_size;
}

int ffw_muxer_init(
    Muxer* muxer,
    AVIOContext* avio_context,
    AVOutputFormat* format,
    AVDictionary** options) {
    int ret;

    muxer->fc->pb = avio_context;
    muxer->fc->oformat = format;

    ret = avformat_write_header(muxer->fc, options);
    if (ret < 0) {
        return ret;
    }

    muxer->initialized = 1;

    return ret;
}

int ffw_muxer_write_frame(Muxer* muxer, AVPacket* packet) {
    return av_write_frame(muxer->fc, packet);
}

int ffw_muxer_interleaved_write_frame(Muxer* muxer, AVPacket* packet) {
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
}
