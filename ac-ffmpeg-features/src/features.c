#include <stdint.h>

#include <libavutil/avutil.h>

typedef void feature_callback_t(void* ctx, const char* feature);

void get_ffmpeg_features(void* ctx, uint8_t all, feature_callback_t* cb) {
    if (all || LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(57, 24, 0)) {
        cb(ctx, "channel_layout_v2");
    }
}
