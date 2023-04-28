#include <libavfilter/avfilter.h>
#include <libavformat/avformat.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>

AVFilterGraph* ffw_filtergraph_new() {
    return avfilter_graph_alloc();
}

int ffw_filtersource_new(AVFilterContext** filter_ctx, AVFilterGraph* filter_graph, AVCodecParameters* codec_params, int tb_num, int tb_den) {
    /* init buffer source: frames from the decoder will be inserted here. */
    char args[512];
    snprintf(args, sizeof(args),
        "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=%d/%d",
        codec_params->width, codec_params->height, codec_params->format,
        tb_num, tb_den,
        codec_params->sample_aspect_ratio.num, codec_params->sample_aspect_ratio.den);
    return avfilter_graph_create_filter(filter_ctx, avfilter_get_by_name("buffer"), "in", args, NULL, filter_graph);
}

int ffw_filtersink_new(AVFilterContext** filter_ctx, AVFilterGraph* filter_graph) {
    /* init buffer sink to terminate the filter chain. */
    return avfilter_graph_create_filter(filter_ctx, avfilter_get_by_name("buffersink"), "out", NULL, NULL, filter_graph);
}

int ffw_filtergraph_init(AVFilterGraph* filter_graph,
    AVFilterContext* buffersrc_ctx, AVFilterContext* buffersink_ctx,
    const char* filters_descr) {
    int ret = 0;
    AVFilterInOut* outputs = avfilter_inout_alloc();
    AVFilterInOut* inputs = avfilter_inout_alloc();

    /*
     * Set the endpoints for the filter graph. The filter_graph will
     * be linked to the graph described by filters_descr.
     */

     /*
      * The buffer source output must be connected to the input pad of
      * the first filter described by filters_descr; since the first
      * filter input label is not specified, it is set to "in" by
      * default.
      */
    outputs->name = av_strdup("in");
    outputs->filter_ctx = buffersrc_ctx;
    outputs->pad_idx = 0;
    outputs->next = NULL;

    /*
     * The buffer sink input must be connected to the output pad of
     * the last filter described by filters_descr; since the last
     * filter output label is not specified, it is set to "out" by
     * default.
     */
    inputs->name = av_strdup("out");
    inputs->filter_ctx = buffersink_ctx;
    inputs->pad_idx = 0;
    inputs->next = NULL;

    ret = avfilter_graph_parse_ptr(filter_graph, filters_descr, &inputs, &outputs, NULL);
    if (ret < 0) {
        return ret;
    }

    ret = avfilter_graph_config(filter_graph, NULL);
    if (ret < 0) {
        return ret;
    }

    return ret;
}

int ffw_filtergraph_push_frame(AVFilterContext* context, AVFrame* frame) {
    int ret = av_buffersrc_add_frame(context, frame);

    if (ret == 0 || ret == AVERROR_EOF) {
        return 1;
    }
    else if (ret == AVERROR(EAGAIN)) {
        return 0;
    }

    return ret;
}

int ffw_filtergraph_take_frame(AVFilterContext* context, AVFrame** out) {
    AVFrame* frame = av_frame_alloc();
    int ret = av_buffersink_get_frame(context, frame);

    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
        return 0;
    }
    else if (ret < 0) {
        return ret;
    }

    *out = frame;

    return 1;
}

void ffw_filtergraph_free(AVFilterGraph* filter_graph) {
    avfilter_graph_free(&filter_graph);
}
