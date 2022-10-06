#include <libavcodec/avcodec.h>

// BSF definitions have been moved into libavcodec/bsf.h in FFmpeg 5.x
#if LIBAVCODEC_VERSION_MAJOR > 58
#include <libavcodec/bsf.h>
#endif

int ffw_bsf_new(const char* name, AVBSFContext** context) {
    const AVBitStreamFilter* filter;
    AVBSFContext* ctx;
    int ret;

    filter = av_bsf_get_by_name(name);
    if (!filter) {
        return AVERROR(EINVAL);
    }

    ret = av_bsf_alloc(filter, &ctx);
    if (ret < 0) {
        return ret;
    }

    ctx->time_base_in.num = 1;
    ctx->time_base_in.den = 1000000;
    ctx->time_base_out.num = 1;
    ctx->time_base_out.den = 1000000;

    *context = ctx;

    return ret;
}

int ffw_bsf_set_input_codec_parameters(AVBSFContext* context, const AVCodecParameters* params) {
    return avcodec_parameters_copy(context->par_in, params);
}

int ffw_bsf_set_output_codec_parameters(AVBSFContext* context, const AVCodecParameters* params) {
    return avcodec_parameters_copy(context->par_out, params);
}

int ffw_bsf_init(AVBSFContext* context, int itb_num, int itb_den, int otb_num, int otb_den) {
    context->time_base_in.num = itb_num;
    context->time_base_in.den = itb_den;
    context->time_base_out.num = otb_num;
    context->time_base_out.den = otb_den;

    return av_bsf_init(context);
}

int ffw_bsf_push(AVBSFContext* context, AVPacket* packet) {
    return av_bsf_send_packet(context, packet);
}

int ffw_bsf_flush(AVBSFContext* context) {
    return av_bsf_send_packet(context, NULL);
}

int ffw_bsf_take(AVBSFContext* context, AVPacket** packet) {
    AVPacket* pkt;
    int ret;

    pkt = av_packet_alloc();
    if (!pkt) {
        return AVERROR(ENOMEM);
    }

    ret = av_bsf_receive_packet(context, pkt);
    if (ret < 0) {
        goto err;
    }

    *packet = pkt;

    return ret;

err:
    av_packet_free(&pkt);

    return ret;
}

void ffw_bsf_free(AVBSFContext* context) {
    av_bsf_free(&context);
}
