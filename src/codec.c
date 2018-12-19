#include <libavcodec/avcodec.h>

static const AVCodec* ffw_find_video_codec(const char* name) {
    const AVCodec* codec;
    void* i = NULL;

    if (name == NULL) {
        return NULL;
    }

    while ((codec = av_codec_iterate(&i))) {
        if (codec->type == AVMEDIA_TYPE_VIDEO) {
            if (strcmp(name, codec->name) == 0) {
                return codec;
            }
        }
    }

    return NULL;
}

AVCodecParameters* ffw_video_codec_parameters_new(const char* codec_name) {
    AVCodecParameters* res;
    const AVCodec* codec;

    codec = ffw_find_video_codec(codec_name);
    if (codec == NULL) {
        return NULL;
    }

    res = avcodec_parameters_alloc();
    if (res == NULL) {
        return NULL;
    }

    res->codec_type = codec->type;
    res->codec_id = codec->id;

    return res;
}

AVCodecParameters* ffw_codec_parameters_clone(const AVCodecParameters* src) {
    AVCodecParameters* res = avcodec_parameters_alloc();
    if (res == NULL) {
        return NULL;
    }

    if (avcodec_parameters_copy(res, src) < 0) {
        goto err;
    }

    return res;

err:
    avcodec_parameters_free(&res);

    return NULL;
}

void ffw_codec_parameters_set_width(AVCodecParameters* params, int width) {
    params->width = width;
}

void ffw_codec_parameters_set_height(AVCodecParameters* params, int height) {
    params->height = height;
}

int ffw_codec_parameters_set_extradata(AVCodecParameters* params, const uint8_t* extradata, int size) {
    if (params->extradata) {
        av_freep(&params->extradata);
    }

    params->extradata_size = 0;

    if (extradata == NULL || size <= 0) {
        params->extradata = NULL;
    } else {
        params->extradata = av_mallocz(size + AV_INPUT_BUFFER_PADDING_SIZE);
        if (params->extradata == NULL) {
            return -1;
        }

        memcpy(params->extradata, extradata, size);

        params->extradata_size = size;
    }

    return 0;
}

void ffw_codec_parameters_free(AVCodecParameters* params) {
    avcodec_parameters_free(&params);
}
