#include<libavutil/channel_layout.h>
#include<libavutil/frame.h>
#include<libavutil/samplefmt.h>

uint64_t ffw_get_channel_layout_by_name(const char* name) {
    return av_get_channel_layout(name);
}

uint64_t ffw_get_default_channel_layour(int channels) {
    return av_get_default_channel_layout(channels);
}

int ffw_get_sample_format_by_name(const char* name) {
    return av_get_sample_fmt(name);
}

int ffw_sample_format_is_none(int format) {
    return format == AV_SAMPLE_FMT_NONE;
}

AVFrame* ffw_frame_new_silence(uint64_t, int, int);
void ffw_frame_free(AVFrame*);

AVFrame* ffw_frame_new_silence(uint64_t channel_layout, int sample_fmt, int nb_samples) {
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

int ffw_frame_format(const AVFrame* frame) {
    return frame->format;
}

int ffw_frame_width(const AVFrame* frame) {
    return frame->width;
}

int ffw_frame_height(const AVFrame* frame) {
    return frame->height;
}

int ffw_frame_nb_samples(const AVFrame* frame) {
    return frame->nb_samples;
}

int ffw_frame_channels(const AVFrame* frame) {
    return frame->channels;
}

uint64_t ffw_frame_channel_layout(const AVFrame* frame) {
    return frame->channel_layout;
}

int64_t ffw_frame_pts(const AVFrame* frame) {
    return frame->pts;
}

AVFrame* ffw_frame_clone(const AVFrame* frame) {
    return av_frame_clone(frame);
}

void ffw_frame_free(AVFrame* frame) {
    av_frame_free(&frame);
}
