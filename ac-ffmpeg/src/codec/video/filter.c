#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>

/**
 * @brief Create a new filter graph.
 */
AVFilterGraph* ffw_filtergraph_new() {
    return avfilter_graph_alloc();
}

/**
 * @brief Initialize a buffer source (frames will be inserted here) and put it
 *   into the filter graph.
 *
 * @param filter_ctx the filter context (output)
 * @param filter_graph the filter graph
 * @param src_args the source buffer arguments (e.g. video frame size, pixel
 *   aspect ratio, etc.)
 * @return int negative AVERROR error code on failure, a non-negative value
 *   otherwise
 */
int ffw_filtersource_new(
    AVFilterContext** filter_ctx,
    AVFilterGraph* filter_graph,
    const char* src_args)
{
    const AVFilter* filter;

    if (!(filter = avfilter_get_by_name("buffer"))) {
        return AVERROR_FILTER_NOT_FOUND;
    }

    return avfilter_graph_create_filter(filter_ctx, filter, "in", src_args, NULL, filter_graph);
}

/**
 * @brief Initialize a buffer sink to terminate the filter chain and put it
 *   into the filter graph.
 *
 * @param filter_ctx the filter context (output)
 * @param filter_graph the filter graph
 * @return int negative AVERROR error code on failure, a non-negative value otherwise
 */
int ffw_filtersink_new(AVFilterContext** filter_ctx, AVFilterGraph* filter_graph) {
    const AVFilter* filter;

    if (!(filter = avfilter_get_by_name("buffersink"))) {
        return AVERROR_FILTER_NOT_FOUND;
    }

    return avfilter_graph_create_filter(filter_ctx, filter, "out", NULL, NULL, filter_graph);
}

/**
 * @brief Initialize the filter graph with a given filter description.
 *
 * @param filter_graph the filter graph
 * @param buffersrc_ctx the buffer source filter context
 * @param buffersink_ctx the buffer sink filter context
 * @param filters_descr the filter description
 * @return int negative AVERROR error code on failure, a non-negative value
 *   otherwise
 */
int ffw_filtergraph_init(
    AVFilterGraph* filter_graph,
    AVFilterContext* buffersrc_ctx,
    AVFilterContext* buffersink_ctx,
    const char* filters_descr)
{
    AVFilterInOut* outputs;
    AVFilterInOut* inputs;
    int ret;

    outputs = NULL;
    inputs = NULL;

    if (!(outputs = avfilter_inout_alloc())) {
        ret = AVERROR(ENOMEM); goto end;
    }

    if (!(inputs = avfilter_inout_alloc())) {
        ret = AVERROR(ENOMEM); goto end;
    }

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

    if ((ret = avfilter_graph_parse_ptr(filter_graph, filters_descr, &inputs, &outputs, NULL)) < 0) {
        goto end;
    }

    ret = avfilter_graph_config(filter_graph, NULL);

end:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);

    return ret;
}

/**
 * @brief Push a given frame into the filter graph.
 *
 * @param context buffer source filter context
 * @param frame frame to push
 * @return int negative AVERROR error code on failure, 0 if the frame wasn't
 *   consumed and the function should be called again later, 1 if the frame was
 *   successfully pushed or on EOF
 */
int ffw_filtergraph_push_frame(AVFilterContext* context, AVFrame* frame) {
    int ret = av_buffersrc_add_frame(context, frame);

    if (ret == 0 || ret == AVERROR_EOF) {
        return 1;
    } else if (ret == AVERROR(EAGAIN)) {
        return 0;
    }

    return ret;
}

/**
 * @brief Take the next frame from the filter graph.
 *
 * @param context buffer sink filter context
 * @param out output frame (output)
 * @return int negative AVERROR error code on failure, 0 if no frame is
 *   currently available, 1 if a frame was returned
 */
int ffw_filtergraph_take_frame(AVFilterContext* context, AVFrame** out) {
    AVFrame* frame;
    int ret;

    if (!(frame = av_frame_alloc())) {
        return AVERROR(ENOMEM);
    }

    ret = av_buffersink_get_frame(context, frame);

    if (ret == AVERROR_EOF || ret == AVERROR(EAGAIN)) {
        ret = 0;
    } else if (ret >= 0) {
        ret = 1;
    }

    if (ret > 0) {
        *out = frame;
    } else {
        av_frame_free(&frame);
    }

    return ret;
}

/**
 * @brief Free the filter graph and all its components.
 *
 * @param filter_graph the filter graph to free
 */
void ffw_filtergraph_free(AVFilterGraph* filter_graph) {
    avfilter_graph_free(&filter_graph);
}
