#include <libavutil/frame.h>
#include <libswscale/swscale.h>

#include <stdlib.h>

typedef struct FrameScaler {
    struct SwsContext* scale_context;
    struct AVFrame* frame;

    int tformat;
    int twidth;
    int theight;
} FrameScaler;

FrameScaler* ffw_frame_scaler_new(
    int sformat, int swidth, int sheight,
    int tformat, int twidth, int theight,
    int flags);

AVFrame* ffw_frame_scaler_scale(FrameScaler* scaler, const AVFrame* src);
void ffw_frame_scaler_free(FrameScaler* scaler);
int ffw_alg_id_to_flags(size_t id);

static AVFrame* alloc_frame(int format, int width, int height) {
    AVFrame* frame = av_frame_alloc();

    if (frame == NULL) {
        return NULL;
    }

    frame->format = format;
    frame->width = width;
    frame->height = height;

    if (av_frame_get_buffer(frame, 0) != 0) {
        av_frame_free(&frame);
    }

    return frame;
}

FrameScaler* ffw_frame_scaler_new(
    int sformat, int swidth, int sheight,
    int tformat, int twidth, int theight,
    int flags) {
    FrameScaler* res = malloc(sizeof(FrameScaler));
    if (res == NULL) {
        return NULL;
    }

    res->scale_context = NULL;
    res->frame = NULL;

    res->tformat = tformat;
    res->twidth = twidth;
    res->theight = theight;

    res->scale_context = sws_getContext(
        swidth, sheight, sformat,
        twidth, theight, tformat,
        flags, NULL, NULL, NULL);

    if (res->scale_context == NULL) {
        goto err;
    }

    return res;

err:
    ffw_frame_scaler_free(res);

    return NULL;
}

AVFrame* ffw_frame_scaler_scale(FrameScaler* scaler, const AVFrame* src) {
    if (scaler->frame == NULL || !av_frame_is_writable(scaler->frame)) {
        // the internal frame has not been initialized yet or someone holds a
        // reference to it, we need to drop it and create a new one
        av_frame_free(&scaler->frame);

        scaler->frame = alloc_frame(scaler->tformat, scaler->twidth, scaler->theight);

        if (scaler->frame == NULL) {
            return NULL;
        }
    }

    AVFrame* dst = scaler->frame;

    dst->pts = src->pts;

    sws_scale(scaler->scale_context,
        (const uint8_t* const*)src->data, src->linesize, 0, src->height,
        dst->data, dst->linesize);

    return av_frame_clone(dst);
}

void ffw_frame_scaler_free(FrameScaler* scaler) {
    if (scaler == NULL) {
        return;
    }

    av_frame_free(&scaler->frame);
    sws_freeContext(scaler->scale_context);
    free(scaler);
}

#define ALG_ID_FAST_BILINEAR 0
#define ALG_ID_BILINEAR      1
#define ALG_ID_BICUBIC       2

int ffw_alg_id_to_flags(size_t id) {
    switch (id) {
        case ALG_ID_FAST_BILINEAR: return SWS_FAST_BILINEAR;
        case ALG_ID_BILINEAR:      return SWS_BILINEAR;
        case ALG_ID_BICUBIC:       return SWS_BICUBIC;
        default: return 0;
    }
}
