#include <stdio.h>

#include "ffmpeg-features.h"

#define CHECK_FEATURE(f)    defined(FFW_FEATURE_ ## f) || defined(PRINT_ALL_FEATURES)

int main(void) {
#if CHECK_FEATURE(CHANNEL_LAYOUT_V2)
    printf("channel_layout_v2\n");
#endif

    return 0;
}
