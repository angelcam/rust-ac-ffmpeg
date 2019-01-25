#include<libswresample/swresample.h>

typedef struct AudioResampler {
    struct SwrContext* resample_context;
    struct AVFrame* tmp_frame;
    struct AVFrame* output_frame;

    uint64_t target_channel_layout;
    int target_channels;
    int target_sample_format;
    int target_sample_rate;
    int target_frame_samples;
    int source_sample_rate;
    int tmp_frame_capacity;

    int64_t source_samples;
    int64_t expected_source_pts;
    int64_t output_samples;
    int64_t input_pts_offset;
    int64_t output_pts_offset;

    int offset;
    int flush;
} AudioResampler;

static AVFrame* alloc_frame(
    uint64_t channel_layout,
    int sample_fmt,
    int sample_rate,
    int nb_samples)
{
    AVFrame* frame;
    int channels;

    frame = av_frame_alloc();
    if (!frame) {
        return NULL;
    }

    channels = av_get_channel_layout_nb_channels(channel_layout);

    frame->channel_layout = channel_layout;
    frame->channels = channels;
    frame->format = sample_fmt;
    frame->sample_rate = sample_rate;
    frame->nb_samples = nb_samples;

    if (av_frame_get_buffer(frame, 0) != 0) {
        av_frame_free(&frame);
    }

    return frame;
}

AudioResampler* ffw_audio_resampler_new(
    uint64_t target_channel_layout,
    int target_sample_format,
    int target_sample_rate,
    int target_frame_samples,
    uint64_t source_channel_layout,
    int source_sample_format,
    int source_sample_rate);

void ffw_audio_resampler_free(AudioResampler* resampler);

AudioResampler* ffw_audio_resampler_new(
    uint64_t target_channel_layout,
    int target_sample_format,
    int target_sample_rate,
    int target_frame_samples,
    uint64_t source_channel_layout,
    int source_sample_format,
    int source_sample_rate) {
    AudioResampler* res = malloc(sizeof(AudioResampler));
    if (!res) {
        return NULL;
    }

    res->resample_context = NULL;
    res->tmp_frame = NULL;
    res->output_frame = NULL;

    res->target_channel_layout = target_channel_layout;
    res->target_channels = av_get_channel_layout_nb_channels(target_channel_layout);
    res->target_sample_format = target_sample_format;
    res->target_sample_rate = target_sample_rate;
    res->target_frame_samples = target_frame_samples;
    res->source_sample_rate = source_sample_rate;
    res->tmp_frame_capacity = 0;

    res->source_samples = 0;
    res->expected_source_pts = 0;
    res->output_samples = 0;
    res->input_pts_offset = 0;
    res->output_pts_offset = 0;

    res->offset = 0;
    res->flush = 0;

    res->resample_context = swr_alloc_set_opts(
        NULL,
        target_channel_layout,
        target_sample_format,
        target_sample_rate,
        source_channel_layout,
        source_sample_format,
        source_sample_rate,
        0,
        NULL);

    if (!res->resample_context) {
        goto err;
    }

    if (swr_init(res->resample_context) < 0) {
        goto err;
    }

    return res;

err:
    ffw_audio_resampler_free(res);

    return NULL;
}

int ffw_audio_resampler_push_frame(AudioResampler* resampler, const AVFrame* frame) {
    int required_capacity;
    int ret;

    int64_t pts_delta;

    // check if the internal frame has been consumed
    if (resampler->tmp_frame && (resampler->offset < resampler->tmp_frame->nb_samples)) {
        return 0;
    }

    if (frame) {
        // set the initial pts expectation
        if (resampler->source_samples == 0) {
            resampler->expected_source_pts = frame->pts;
            resampler->input_pts_offset = frame->pts;
            resampler->output_pts_offset = frame->pts
                * resampler->target_sample_rate
                / resampler->source_sample_rate;
        }

        pts_delta = frame->pts - resampler->expected_source_pts;

        if (pts_delta > 0) {
            // insert a given number of silence samples if there is a gap
            ret = swr_inject_silence(resampler->resample_context, pts_delta);

            if (ret < 0) {
                return ret;
            }
        } else if (pts_delta < 0) {
            // drop a given number of samples
            ret = swr_drop_output(
                resampler->resample_context,
                -pts_delta * resampler->target_sample_rate / resampler->source_sample_rate);

            if (ret < 0) {
                return ret;
            }
        }

        resampler->source_samples += frame->nb_samples + pts_delta;
        resampler->expected_source_pts = resampler->input_pts_offset + resampler->source_samples;

        required_capacity = swr_get_out_samples(
            resampler->resample_context,
            frame->nb_samples);
    } else {
        resampler->flush = 1;

        required_capacity = swr_get_delay(
                resampler->resample_context,
                resampler->target_sample_rate) + 3;
    }

    if (required_capacity < 0) {
        return required_capacity;
    }

    if (!resampler->tmp_frame
        || !av_frame_is_writable(resampler->tmp_frame)
        || (required_capacity > resampler->tmp_frame_capacity)) {
        av_frame_free(&resampler->tmp_frame);

        resampler->tmp_frame = alloc_frame(
            resampler->target_channel_layout,
            resampler->target_sample_format,
            resampler->target_sample_rate,
            required_capacity);

        if (!resampler->tmp_frame) {
            return -1;
        }

        resampler->tmp_frame_capacity = required_capacity;
    }

    resampler->tmp_frame->nb_samples = 0;
    resampler->offset = 0;

    ret = swr_convert_frame(resampler->resample_context, resampler->tmp_frame, frame);

    if (ret < 0) {
        return ret;
    }

    resampler->tmp_frame->pts = resampler->output_samples + resampler->output_pts_offset;
    resampler->output_samples += resampler->tmp_frame->nb_samples;

    return 1;
}

int ffw_audio_resampler_take_frame(AudioResampler* resampler, AVFrame** frame) {
    int available_samples;
    int required_samples;
    int copy_samples;

    // sanity check
    if (!resampler->tmp_frame) {
        return 0;
    }

    // for non-fixed target frame size, we can just clone the tmp frame
    if (!resampler->target_frame_samples) {
        // reset the flush flag
        resampler->flush = 0;

        if (resampler->tmp_frame->nb_samples > 0) {
            *frame = av_frame_clone(resampler->tmp_frame);
            resampler->tmp_frame->nb_samples = 0;
            return 1;
        } else {
            return 0;
        }
    }

    // check if the internal frame is initialized and writable and create a new
    // one if necessary
    if (!resampler->output_frame || !av_frame_is_writable(resampler->output_frame)) {
        av_frame_free(&resampler->output_frame);

        resampler->output_frame = alloc_frame(
            resampler->target_channel_layout,
            resampler->target_sample_format,
            resampler->target_sample_rate,
            resampler->target_frame_samples);

        if (!resampler->output_frame) {
            return -1;
        }

        resampler->output_frame->nb_samples = 0;
    }

    required_samples = resampler->target_frame_samples - resampler->output_frame->nb_samples;
    available_samples = resampler->tmp_frame->nb_samples - resampler->offset;

    // check how much we can copy
    if (available_samples > required_samples) {
        copy_samples = required_samples;
    } else {
        copy_samples = available_samples;
    }

    // copy the samples
    if (copy_samples > 0) {
        av_samples_copy(
            resampler->output_frame->extended_data,
            resampler->tmp_frame->extended_data,
            resampler->output_frame->nb_samples,
            resampler->offset,
            copy_samples,
            resampler->target_channels,
            resampler->target_sample_format);

        // set output frame pts
        if (!resampler->output_frame->nb_samples) {
            resampler->output_frame->pts = resampler->tmp_frame->pts + resampler->offset;
        }

        resampler->offset += copy_samples;
        resampler->output_frame->nb_samples += copy_samples;
    }

    if (!resampler->flush) {
        if (resampler->output_frame->nb_samples < resampler->target_frame_samples) {
            return 0;
        }
    }

    *frame = av_frame_clone(resampler->output_frame);

    // reuse the output frame
    resampler->output_frame->nb_samples = 0;

    // reset the flush flag
    if (resampler->offset >= resampler->tmp_frame->nb_samples) {
        resampler->flush = 0;
    }

    return 1;
}

void ffw_audio_resampler_free(AudioResampler* resampler) {
    if (!resampler) {
        return;
    }

    av_frame_free(&resampler->tmp_frame);
    av_frame_free(&resampler->output_frame);
    swr_free(&resampler->resample_context);

    free(resampler);
}
