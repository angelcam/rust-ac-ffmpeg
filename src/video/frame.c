#include<libavutil/frame.h>

int ffw_frame_format(const AVFrame* frame) {
    return frame->format;
}

int ffw_frame_width(const AVFrame* frame) {
    return frame->width;
}

int ffw_frame_height(const AVFrame* frame) {
    return frame->height;
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
