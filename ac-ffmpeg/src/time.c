#include <libavutil/avutil.h>
#include <libavutil/mathematics.h>

int64_t ffw_null_timestamp = AV_NOPTS_VALUE;

int64_t ffw_rescale_q(int64_t n, int32_t aq_num, int32_t aq_den, int32_t bq_num, int32_t bq_den) {
    int64_t a = (int64_t)aq_num * (int64_t)bq_den;
    int64_t b = (int64_t)bq_num * (int64_t)aq_den;

    return av_rescale_rnd(n, a, b, AV_ROUND_ZERO);
}
