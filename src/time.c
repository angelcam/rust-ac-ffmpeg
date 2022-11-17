#include <libavutil/avutil.h>
#include <libavutil/mathematics.h>

int64_t ffw_rescale_q(int64_t n, int aq_num, int aq_den, int bq_num, int bq_den) {
    int64_t a = aq_num * (int64_t)bq_den;
    int64_t b = bq_num * (int64_t)aq_den;

    return av_rescale_rnd(n, a, b, AV_ROUND_ZERO);
}

int64_t ffw_null_timestamp() {
    return AV_NOPTS_VALUE;
}
