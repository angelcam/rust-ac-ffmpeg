#ifndef _FFW_FEATURES_H
#define _FFW_FEATURES_H

#include <libavutil/avutil.h>

// available since FFmpeg 5.1
#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(57, 24, 0)
#define FFW_FEATURE_CHANNEL_LAYOUT_V2
#endif

#endif
