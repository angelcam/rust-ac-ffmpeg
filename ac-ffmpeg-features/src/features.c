#include <stdint.h>

#include <libavutil/avutil.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>

typedef void feature_callback_t(void* ctx, const char* feature);

void get_ffmpeg_features(void* ctx, uint8_t all, feature_callback_t* cb) {
    if (all || LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(57, 24, 0)) {
        cb(ctx, "channel_layout_v2");
    }

    if (all || LIBAVCODEC_VERSION_INT >= AV_VERSION_INT(60, 30, 0)) {
        cb(ctx, "codec_params_side_data");
    }

    if (all || LIBAVFORMAT_VERSION_INT < AV_VERSION_INT(60, 15, 0)) {
        cb(ctx, "stream_side_data");
    }
}
