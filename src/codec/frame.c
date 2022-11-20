#include <libavutil/avutil.h>
#include <libavutil/channel_layout.h>
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixdesc.h>
#include <libavutil/pixfmt.h>
#include <libavutil/samplefmt.h>

#include "ffmpeg-features.h"

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2

int ffw_channel_layout_get_default(AVChannelLayout** layout, uint32_t channels) {
    AVChannelLayout* res;

    if (!(res = calloc(1, sizeof(AVChannelLayout)))) {
        return AVERROR(ENOMEM);
    }

    av_channel_layout_default(res, channels);

    *layout = res;

    return 0;
}

int ffw_channel_layout_from_string(AVChannelLayout** layout, const char* str) {
    AVChannelLayout* res;
    int ret;

    if (!(res = calloc(1, sizeof(AVChannelLayout)))) {
        return AVERROR(ENOMEM);
    }

    if ((ret = av_channel_layout_from_string(res, str)) != 0) {
        goto err;
    }

    *layout = res;

    return 0;

err:
    free(res);

    return ret;
}

int ffw_channel_layout_clone(AVChannelLayout** dst, const AVChannelLayout* src) {
    AVChannelLayout* res;
    int ret;

    if (!(res = calloc(1, sizeof(AVChannelLayout)))) {
        return AVERROR(ENOMEM);
    }

    if ((ret = av_channel_layout_copy(res, src)) != 0) {
        goto err;
    }

    *dst = res;

    return 0;

err:
    free(res);

    return ret;
}

int ffw_channel_layout_is_valid(const AVChannelLayout* layout) {
    return av_channel_layout_check(layout);
}

uint32_t ffw_channel_layout_get_channels(const AVChannelLayout* layout) {
    return layout->nb_channels;
}

int ffw_channel_layout_compare(const AVChannelLayout* a, const AVChannelLayout* b) {
    return av_channel_layout_compare(a, b);
}

void ffw_channel_layout_free(AVChannelLayout* layout) {
    if (!layout) {
        return;
    }

    av_channel_layout_uninit(layout);

    free(layout);
}

#else // FFW_FEATURE_CHANNEL_LAYOUT_V2

uint64_t ffw_get_channel_layout_by_name(const char* name) {
    return av_get_channel_layout(name);
}

uint64_t ffw_get_default_channel_layout(int channels) {
    return av_get_default_channel_layout(channels);
}

int ffw_get_channel_layout_channels(uint64_t layout) {
    return av_get_channel_layout_nb_channels(layout);
}

#endif // FFW_FEATURE_CHANNEL_LAYOUT_V2

int ffw_get_sample_format_by_name(const char* name) {
    return av_get_sample_fmt(name);
}

const char* ffw_get_sample_format_name(int format) {
    return av_get_sample_fmt_name(format);
}

int ffw_sample_format_is_planar(int format) {
    return av_sample_fmt_is_planar(format);
}

int ffw_sample_format_is_none(int format) {
    return format == AV_SAMPLE_FMT_NONE;
}

int ffw_get_pixel_format_by_name(const char* name) {
    return av_get_pix_fmt(name);
}

const char* ffw_get_pixel_format_name(int format) {
    return av_get_pix_fmt_name(format);
}

int ffw_pixel_format_is_none(int format) {
    return format == AV_PIX_FMT_NONE;
}

AVFrame* ffw_frame_new_black(int, int, int);
void ffw_frame_free(AVFrame*);

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
AVFrame* ffw_frame_new_silence(const AVChannelLayout* channel_layout, int sample_fmt, int sample_rate, int nb_samples) {
    AVFrame* frame = av_frame_alloc();

    if (!frame) {
        return NULL;
    }

    frame->format = sample_fmt;
    frame->sample_rate = sample_rate;
    frame->nb_samples = nb_samples;

    if (av_channel_layout_copy(&frame->ch_layout, channel_layout) != 0) {
        goto err;
    }

    if (av_frame_get_buffer(frame, 0) != 0) {
        goto err;
    }

    av_samples_set_silence(frame->extended_data, 0, nb_samples, frame->ch_layout.nb_channels, sample_fmt);

    return frame;

err:
    ffw_frame_free(frame);

    return NULL;
}
#else // FFW_FEATURE_CHANNEL_LAYOUT_V2
AVFrame* ffw_frame_new_silence(const uint64_t* channel_layout, int sample_fmt, int sample_rate, int nb_samples) {
    AVFrame* frame;
    int channels;

    if (!(frame = av_frame_alloc())) {
        return NULL;
    }

    channels = av_get_channel_layout_nb_channels(*channel_layout);

    frame->channel_layout = *channel_layout;
    frame->channels = channels;
    frame->format = sample_fmt;
    frame->sample_rate = sample_rate;
    frame->nb_samples = nb_samples;

    if (av_frame_get_buffer(frame, 0) != 0) {
        goto err;
    }

    av_samples_set_silence(frame->extended_data, 0, nb_samples, channels, sample_fmt);

    return frame;

err:
    ffw_frame_free(frame);

    return NULL;
}
#endif // FFW_FEATURE_CHANNEL_LAYOUT_V2

AVFrame* ffw_frame_new_black(int pixel_format, int width, int height) {
    AVFrame* frame;
    uint8_t* data[4];
    ptrdiff_t linesize[4];

    frame = av_frame_alloc();

    if (frame == NULL) {
        return NULL;
    }

    frame->format = pixel_format;
    frame->width = width;
    frame->height = height;

    if (av_frame_get_buffer(frame, 0) != 0) {
        goto err;
    }

    data[0] = frame->data[0];
    data[1] = frame->data[1];
    data[2] = frame->data[2];
    data[3] = frame->data[3];

    linesize[0] = frame->linesize[0];
    linesize[1] = frame->linesize[1];
    linesize[2] = frame->linesize[2];
    linesize[3] = frame->linesize[3];

    if (av_image_fill_black(data, linesize, pixel_format, AVCOL_RANGE_MPEG, width, height) < 0) {
        goto err;
    }

    return frame;

err:
    ffw_frame_free(frame);

    return NULL;
}

int ffw_frame_get_format(const AVFrame* frame) {
    return frame->format;
}

int ffw_frame_get_width(const AVFrame* frame) {
    return frame->width;
}

int ffw_frame_get_height(const AVFrame* frame) {
    return frame->height;
}

int ffw_frame_get_sample_rate(const AVFrame* frame) {
    return frame->sample_rate;
}

int ffw_frame_get_nb_samples(const AVFrame* frame) {
    return frame->nb_samples;
}

#ifdef FFW_FEATURE_CHANNEL_LAYOUT_V2
const AVChannelLayout * ffw_frame_get_channel_layout(const AVFrame* frame) {
    return &frame->ch_layout;
}
#else
const uint64_t * ffw_frame_get_channel_layout(const AVFrame* frame) {
    return &frame->channel_layout;
}
#endif

int64_t ffw_frame_get_best_effort_timestamp(const AVFrame* frame) {
    return frame->best_effort_timestamp;
}

int64_t ffw_frame_get_pts(const AVFrame* frame) {
    return frame->pts;
}

void ffw_frame_set_pts(AVFrame* frame, int64_t pts) {
    frame->pts = pts;
}

AVFrame* ffw_frame_clone(const AVFrame* frame) {
    return av_frame_clone(frame);
}

void ffw_frame_free(AVFrame* frame) {
    av_frame_free(&frame);
}

size_t ffw_frame_get_line_size(const AVFrame* frame, size_t plane) {
    return frame->linesize[plane];
}

size_t ffw_frame_get_line_count(const AVFrame* frame, size_t plane) {
     // XXX: This is a bit hack-ish. Unfortunately, FFmpeg does not have a function for this. The
    // code has been copy-pasted from av_image_fill_pointers().

    const AVPixFmtDescriptor* desc = av_pix_fmt_desc_get(frame->format);

    int s = (plane == 1 || plane == 2) ? desc->log2_chroma_h : 0;
    int h = (frame->height + (1 << s) - 1) >> s;

    return h;
}

uint8_t* ffw_frame_get_plane_data(AVFrame* frame, size_t index) {
    return frame->extended_data[index];
}

int ffw_frame_is_writable(const AVFrame* frame) {
    return av_frame_is_writable((AVFrame*)frame);
}

int ffw_frame_make_writable(AVFrame* frame) {
    return av_frame_make_writable(frame);
}

int ffw_frame_get_picture_type(const AVFrame* frame) {
    switch (frame->pict_type) {
        case AV_PICTURE_TYPE_I: return 1;
        case AV_PICTURE_TYPE_P: return 2;
        case AV_PICTURE_TYPE_B: return 3;
        case AV_PICTURE_TYPE_S: return 4;
        case AV_PICTURE_TYPE_SI: return 5;
        case AV_PICTURE_TYPE_SP: return 6;
        case AV_PICTURE_TYPE_BI: return 7;
        default: return 0;
    }
}

void ffw_frame_set_picture_type(AVFrame* frame, int picture_type) {
    enum AVPictureType type;

    switch(picture_type) {
        case 1: type = AV_PICTURE_TYPE_I; break;
        case 2: type = AV_PICTURE_TYPE_P; break;
        case 3: type = AV_PICTURE_TYPE_B; break;
        case 4: type = AV_PICTURE_TYPE_S; break;
        case 5: type = AV_PICTURE_TYPE_SI; break;
        case 6: type = AV_PICTURE_TYPE_SP; break;
        case 7: type = AV_PICTURE_TYPE_BI; break;
        default: type = AV_PICTURE_TYPE_NONE; break;
    }

    frame->pict_type = type;
}
