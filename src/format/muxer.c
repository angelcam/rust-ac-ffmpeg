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
int ffw_muxer_new_stream(Muxer*, const AVCodecParameters*);
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

int ffw_muxer_new_stream(Muxer* muxer, const AVCodecParameters* params) {
    AVStream* s;
    int ret;

    s = avformat_new_stream(muxer->fc, NULL);
    if (s == NULL) {
        return -1;
    }

    ret = avcodec_parameters_copy(s->codecpar, params);
    if (ret < 0) {
        return -1;
    }

    return s->index;
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
