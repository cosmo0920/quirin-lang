# W46 metric aggregate contracts: pre-implementation prediction

Base `207cebe4cdb745f6216ff2b511c4b7fd134581b9`. The intended six
CMetrics scalar getters are `cmt_metric_hist_get_value`,
`cmt_metric_hist_get_count_value`, `cmt_metric_hist_get_sum_value`,
`cmt_summary_quantile_get_value`, `cmt_summary_get_sum_value`, and
`cmt_summary_get_count_value`. Their candidate bodies and public prototypes
were read at Fluent Bit revisions `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`
and `046accf6aded7906f66a1baf0202da349e521736`; the relevant source and
headers are byte-identical between pins. The quantile getter guards null and
index bounds; the histogram bucket getter does not. Neither fact becomes a
resource-lifetime claim.

I predict fewer uncontracted-call leaves, including histogram count and sum
leaves named in the prior census, but no specific whole-function Safe movement.
The six neutral declarations leave caller custody intact and return only
scalars. No new validated Unsafe or release behavior is expected. The pinned
contract is still a reviewed premise, not a proof of the C body; the arbiter
must decide the actual verdict and diagnostic movement.
