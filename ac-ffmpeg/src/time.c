#include <libavutil/avutil.h>
#include <libavutil/mathematics.h>

int64_t ffw_null_timestamp = AV_NOPTS_VALUE;

int64_t ffw_rescale_q(int64_t n, uint32_t aq_num, uint32_t aq_den, uint32_t bq_num, uint32_t bq_den) {
    int64_t a = aq_num * (int64_t)bq_den;
    int64_t b = bq_num * (int64_t)aq_den;

    return av_rescale_rnd(n, a, b, AV_ROUND_ZERO);
}
