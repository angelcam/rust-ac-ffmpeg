#include <libavutil/channel_layout.h>
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixdesc.h>
#include <libavutil/pixfmt.h>
#include <libavutil/samplefmt.h>

uint64_t ffw_get_channel_layout_by_name(const char* name) {
    return av_get_channel_layout(name);
}

uint64_t ffw_get_default_channel_layout(int channels) {
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

AVFrame* ffw_frame_new_silence(uint64_t, int, int, int);
AVFrame* ffw_frame_new_black(int, int, int);
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
