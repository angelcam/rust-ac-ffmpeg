#include<libavutil/channel_layout.h>
#include<libavutil/frame.h>
#include<libavutil/pixdesc.h>
#include<libavutil/pixfmt.h>
#include<libavutil/samplefmt.h>

uint64_t ffw_get_channel_layout_by_name(const char* name) {
    return av_get_channel_layout(name);
}

uint64_t ffw_get_default_channel_layour(int channels) {
    return av_get_default_channel_layout(channels);
}

int ffw_get_channel_layout_channels(uint64_t layout) {
    return av_get_channel_layout_nb_channels(layout);
}

int ffw_get_sample_format_by_name(const char* name) {
    return av_get_sample_fmt(name);
}

const char* ffw_get_sample_format_name(int format) {
    return av_get_sample_fmt_name(format);
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

AVFrame* ffw_frame_new_silence(uint64_t, int, int, int);
void ffw_frame_free(AVFrame*);

AVFrame* ffw_frame_new_silence(uint64_t channel_layout, int sample_fmt, int sample_rate, int nb_samples) {
    AVFrame* frame;
    int channels;

    frame = av_frame_alloc();

    if (frame == NULL) {
        return NULL;
    }

    channels = av_get_channel_layout_nb_channels(channel_layout);

    frame->channel_layout = channel_layout;
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

int ffw_frame_get_channels(const AVFrame* frame) {
    return frame->channels;
}

uint64_t ffw_frame_get_channel_layout(const AVFrame* frame) {
    return frame->channel_layout;
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
