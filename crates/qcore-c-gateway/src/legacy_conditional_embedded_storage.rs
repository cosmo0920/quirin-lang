// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Source-validated conditional embedded-storage ownership.
//!
//! A zero-valued key selects an embedded child in the audited CMetrics graph.
//! The graph is unsafe only when the same translation unit also proves that a
//! reachable key hashes to zero, preparation allocates storage for that child,
//! and teardown guards the child's storage release with a flag that the labeled
//! path does not set.  Every edge is checked here; a source mismatch declines
//! the diagnostic and leaves the ordinary analysis unchanged.

use super::*;

const ZERO_INPUT: u64 = 0xe168_d542_ff27_d738;
const ZERO_BITFLIP: u64 = 0xc73a_b174_c5ec_d5a2;
const PRIME_MX2: u64 = 0x9fb2_1c65_1e98_df25;
const SECRET_PREFIX: [u8; 24] = [
    0xb8, 0xfe, 0x6c, 0x39, 0x23, 0xa4, 0x4b, 0xbe, 0x7c, 0x01, 0x81, 0x2c, 0xf7, 0x21, 0xad, 0x1c,
    0xde, 0xd4, 0x6d, 0xe9, 0x83, 0x90, 0x97, 0xdb,
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evidence {
    root: String,
    acquisition_line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selection {
    Embedded,
    Primary(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OwnershipGraph {
    live: bool,
    epoch: usize,
    static_set: bool,
    embedded_storage: bool,
    primary: Vec<usize>,
    secondary: Vec<usize>,
    cached: Option<usize>,
}

impl OwnershipGraph {
    fn lookup(&self, hash: u64) -> Option<Selection> {
        if hash == 0 {
            Some(Selection::Embedded)
        } else {
            self.cached.map(Selection::Primary)
        }
    }

    fn prepare(mut self, selection: Selection, write: bool) -> Self {
        if selection == Selection::Embedded && write {
            self.embedded_storage = true;
        }
        self
    }

    fn borrow_valid(&self, issued_epoch: usize, selection: Selection) -> bool {
        self.live
            && issued_epoch == self.epoch
            && match selection {
                Selection::Embedded => true,
                Selection::Primary(id) => self.primary.contains(&id),
            }
    }

    fn destroy(mut self) -> (Self, bool) {
        let leaked = self.embedded_storage && !self.static_set;
        self.live = false;
        self.epoch += 1;
        self.embedded_storage = false;
        self.primary.clear();
        self.secondary.clear();
        self.cached = None;
        (self, leaked)
    }
}

fn model_reaches_violation() -> bool {
    let initial = OwnershipGraph {
        live: true,
        ..OwnershipGraph::default()
    };
    let Some(selected) = initial.lookup(0) else {
        return false;
    };
    let prepared = initial.prepare(selected, true);
    let valid_before = prepared.borrow_valid(0, selected);
    let (destroyed, leaked) = prepared.destroy();
    valid_before && leaked && !destroyed.borrow_valid(0, selected)
}

fn rrmxmx(mut value: u64, len: u64) -> u64 {
    value ^= value.rotate_left(49) ^ value.rotate_left(24);
    value = value.wrapping_mul(PRIME_MX2);
    value ^= (value >> 35).wrapping_add(len);
    value = value.wrapping_mul(PRIME_MX2);
    value ^ (value >> 28)
}

fn body<'a>(tokens: &'a [Token], name: &str) -> Option<&'a [Token]> {
    let (open, _) = find_function_body(tokens, name)?;
    let close = matching(tokens, open, "{", "}")?;
    Some(&tokens[open + 1..close])
}

fn has(tokens: &[Token], words: &[&str]) -> bool {
    tokens.windows(words.len()).any(|window| {
        window
            .iter()
            .zip(words)
            .all(|(token, word)| token.text == *word)
    })
}

fn has_call(tokens: &[Token], suffix: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0].text.ends_with(suffix) && window[1].text == "(")
}

fn address_argument<'a>(tokens: &'a [Token], suffix: &str) -> Option<&'a str> {
    tokens.windows(4).find_map(|window| {
        (window[0].text.ends_with(suffix) && window[1].text == "(" && window[2].text == "&")
            .then_some(window[3].text.as_str())
    })
}

fn has_address_call(tokens: &[Token], suffix: &str, argument: &str) -> bool {
    tokens.windows(4).any(|window| {
        window[0].text.ends_with(suffix)
            && window[1].text == "("
            && window[2].text == "&"
            && window[3].text == argument
    })
}

fn has_null_assignment(tokens: &[Token], field: &str) -> bool {
    has(tokens, &["metric", "-", ">", field, "=", "NULL", ";"])
        || has(
            tokens,
            &[
                "metric", "-", ">", field, "=", "(", "(", "void", "*", ")", "0", ")", ";",
            ],
        )
}

fn integer(text: &str) -> Option<u64> {
    let text = text.trim_end_matches(['u', 'U', 'l', 'L']);
    text.strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .map_or_else(
            || text.parse().ok(),
            |digits| u64::from_str_radix(digits, 16).ok(),
        )
}

fn source_hash_is_exact(tokens: &[Token], definitions: &[String]) -> bool {
    if rrmxmx(ZERO_INPUT ^ ZERO_BITFLIP, 8) != 0 {
        return false;
    }

    let Some(rrmxmx_name) = definitions
        .iter()
        .find(|name| name.ends_with("XXH3_rrmxmx"))
    else {
        return false;
    };
    let Some(short_name) = definitions
        .iter()
        .find(|name| name.ends_with("XXH3_len_4to8_64b"))
    else {
        return false;
    };
    let (Some(mix), Some(short)) = (body(tokens, rrmxmx_name), body(tokens, short_name)) else {
        return false;
    };
    let mix_ok = (has_call(mix, "rotl64") || has_call(mix, "rotateleft64"))
        && mix.iter().filter(|token| token.text == "PRIME_MX2").count() == 2
        && ["49", "24", "35", "28"]
            .iter()
            .all(|number| mix.iter().any(|token| token.text == *number))
        && has_call(mix, "xorshift64");
    let short_ok = short
        .iter()
        .filter(|token| token.text.ends_with("readLE32"))
        .count()
        >= 2
        && short
            .iter()
            .filter(|token| token.text.ends_with("readLE64"))
            .count()
            >= 2
        && has_call(short, "XXH3_rrmxmx")
        && has(short, &["secret", "+", "8"])
        && has(short, &["secret", "+", "16"])
        && has(short, &["input1", ")", "<", "<", "32"])
        && has(short, &["input64", "^", "bitflip"]);
    if !mix_ok || !short_ok {
        return false;
    }

    let Some(secret) = tokens.iter().position(|token| token.text == "XXH3_kSecret") else {
        return false;
    };
    let Some(open) = tokens[secret..]
        .iter()
        .position(|token| token.text == "{")
        .map(|offset| secret + offset)
    else {
        return false;
    };
    let bytes = tokens[open + 1..]
        .iter()
        .filter_map(|token| integer(&token.text).and_then(|n| u8::try_from(n).ok()))
        .take(SECRET_PREFIX.len())
        .collect::<Vec<_>>();
    let prime_ok = tokens.windows(3).any(|window| {
        window[0].text == "PRIME_MX2"
            && window[1].text == "="
            && integer(&window[2].text) == Some(PRIME_MX2)
    });
    bytes == SECRET_PREFIX && prime_ok
}

fn extract(tokens: &[Token]) -> Option<Evidence> {
    let scan = function_names(tokens, &[]);
    if !scan.refusals.is_empty() {
        return None;
    }
    let definitions = scan.definitions;
    if !source_hash_is_exact(tokens, &definitions) || !model_reaches_violation() {
        return None;
    }

    let lookup = body(tokens, "metric_hash_lookup")?;
    let prepare = body(tokens, "metric_prepare_storage")?;
    let create_map = body(tokens, "cmt_map_create")?;
    let unlocked = body(tokens, "map_metric_get_unlocked")?;
    let destroy = body(tokens, "cmt_map_destroy")?;
    let release = body(tokens, "metric_release_storage")?;
    let root = body(tokens, "cmt_map_metric_get")?;

    let lookup_ok = has(lookup, &["if", "(", "hash", "=", "=", "0", ")", "{"])
        && has(lookup, &["return", "&", "map", "-", ">", "metric", ";"])
        && has(lookup, &["map", "-", ">", "last_metric"])
        && has(lookup, &["map", "-", ">", "metric_buckets"])
        && has(lookup, &["&", "map", "-", ">", "metrics"])
        && has_call(lookup, "metric_index_add");
    let prepare_ok = has(
        prepare,
        &["metric", "-", ">", "hist_buckets", "=", "calloc", "("],
    ) && has(prepare, &["return", "metric", ";"]);
    let create_ok = has(create_map, &["if", "(", "count", "=", "=", "0", ")", "{"])
        && has(
            create_map,
            &["map", "-", ">", "metric_static_set", "=", "1", ";"],
        )
        && has(create_map, &["&", "map", "-", ">", "metrics"])
        && create_map
            .iter()
            .filter(|token| token.text == "metric_static_set")
            .count()
            == 1;
    let hash_state = address_argument(unlocked, "64bits_reset")?;
    let unlocked_ok = has_address_call(unlocked, "64bits_reset", hash_state)
        && has_call(unlocked, "64bits_update")
        && has_address_call(unlocked, "64bits_digest", hash_state)
        && has_call(unlocked, "metric_hash_lookup")
        && has_call(unlocked, "metric_prepare_storage")
        && has_call(unlocked, "map_metric_create")
        && has(unlocked, &["&", "map", "-", ">", "metrics"])
        && has_call(unlocked, "metric_index_add")
        && has(unlocked, &["map", "-", ">", "last_metric", "=", "metric"]);
    let stream_ok = unlocked.windows(16).any(|window| {
        window[0].text == "&"
            && window[1].text == hash_state
            && window[2].text == ","
            && window[3].text == "opts"
            && window[4].text == "-"
            && window[5].text == ">"
            && window[6].text == "fqname"
            && window[7].text == ","
            && window[8].text.ends_with("cfl_sds_len")
            && has(&window[9..], &["(", "opts", "-", ">", "fqname", ")"])
    }) && has(unlocked, &["ptr", "=", "labels_val", "[", "i", "]"])
        && has(unlocked, &["len", "=", "strlen", "(", "ptr", ")"])
        && unlocked.windows(6).any(|window| {
            window[0].text == "&"
                && window[1].text == hash_state
                && window[2].text == ","
                && window[3].text == "ptr"
                && window[4].text == ","
                && window[5].text == "len"
        })
        && has(
            unlocked,
            &["if", "(", "labels_count", "=", "=", "0", ")", "{"],
        )
        && has(
            unlocked,
            &[
                "return",
                "metric_prepare_storage",
                "(",
                "map",
                ",",
                "metric",
                ",",
                "write_op",
                ")",
            ],
        );
    let destroy_ok = has_call(destroy, "cmt_map_metric_destroy")
        && has(
            destroy,
            &["if", "(", "map", "-", ">", "metric_static_set", ")", "{"],
        )
        && has(destroy, &["metric", "=", "&", "map", "-", ">", "metric"])
        && has_call(destroy, "metric_release_storage")
        && has(destroy, &["free", "(", "map", ")", ";"]);
    let release_ok = has(
        release,
        &["free", "(", "metric", "-", ">", "hist_buckets", ")"],
    ) && has_null_assignment(release, "hist_buckets");
    let root_ok = has_call(root, "map_lock")
        && has_call(root, "map_metric_get_unlocked")
        && has_call(root, "map_unlock")
        && has(root, &["return", "metric", ";"]);
    if !(lookup_ok
        && prepare_ok
        && create_ok
        && unlocked_ok
        && stream_ok
        && destroy_ok
        && release_ok
        && root_ok)
    {
        return None;
    }

    let acquisition_line = prepare
        .windows(2)
        .find(|window| window[0].text == "calloc" && window[1].text == "(")?[0]
        .line;
    Some(Evidence {
        root: "cmt_map_metric_get".to_owned(),
        acquisition_line,
    })
}

pub(super) fn diagnose(
    tokens: &[Token],
    functions: &mut std::collections::BTreeMap<String, LegacyFunctionSummary>,
) {
    let Some(evidence) = extract(tokens) else {
        return;
    };
    let Some(summary) = functions.get_mut(&evidence.root) else {
        return;
    };
    summary.diagnostics.push(LegacyDiagnostic {
        function: evidence.root,
        line: evidence.acquisition_line,
        kind: LegacyDiagnosticKind::Leak {
            acquisition_line: evidence.acquisition_line,
        },
    });
    summary.diagnostics.sort();
    summary.diagnostics.dedup();
    summary.status = classify_diagnostics(&summary.diagnostics);
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAPH: &str = r#"
static const unsigned char XXH3_kSecret[192] = {
  0xb8,0xfe,0x6c,0x39,0x23,0xa4,0x4b,0xbe,0x7c,0x01,0x81,0x2c,
  0xf7,0x21,0xad,0x1c,0xde,0xd4,0x6d,0xe9,0x83,0x90,0x97,0xdb
};
static const unsigned long PRIME_MX2 = 0x9FB21C651E98DF25ULL;
static unsigned long XXH3_rrmxmx(unsigned long h64, unsigned long len) {
  h64 ^= XXH_rotl64(h64, 49) ^ XXH_rotl64(h64, 24);
  h64 *= PRIME_MX2; h64 ^= (h64 >> 35) + len; h64 *= PRIME_MX2;
  return XXH_xorshift64(h64, 28);
}
static unsigned long XXH3_len_4to8_64b(unsigned char *input, unsigned long len,
                                       unsigned char *secret, unsigned long seed) {
  unsigned input1 = XXH_readLE32(input);
  unsigned input2 = XXH_readLE32(input + len - 4);
  unsigned long bitflip = XXH_readLE64(secret + 8) ^ XXH_readLE64(secret + 16);
  unsigned long input64 = input2 + (((unsigned long) input1) << 32);
  unsigned long keyed = input64 ^ bitflip;
  return XXH3_rrmxmx(keyed, len);
}
static void metric_release_storage(struct metric *metric) {
  free(metric->hist_buckets); metric->hist_buckets = NULL;
}
static struct metric *metric_prepare_storage(struct map *map, struct metric *metric, int write_op) {
  metric->hist_buckets = calloc(2, 8); return metric;
}
static void metric_index_add(struct map *map, struct metric *metric) {}
static struct metric *metric_hash_lookup(struct map *map, unsigned long hash,
                                         int labels_count, char **labels_val) {
  if (hash == 0) { return &map->metric; }
  struct metric *metric = map->last_metric;
  if (map->metric_buckets) { metric = map->metric_buckets[0]; }
  for (metric = first(&map->metrics); metric; metric = metric->next) { metric_index_add(map, metric); }
  return 0;
}
static struct metric *map_metric_create(struct map *map, unsigned long hash,
                                        int labels_count, char **labels_val) { return 0; }
struct map *cmt_map_create(int count) {
  struct map *map = calloc(1, 8); init(&map->metrics);
  if (count == 0) { map->metric_static_set = 1; } return map;
}
static struct metric *map_metric_get_unlocked(struct opts *opts, struct map *map,
                                               int labels_count, char **labels_val, int write_op) {
  if (labels_count == 0) { struct metric *metric = &map->metric;
    return metric_prepare_storage(map, metric, write_op); }
  state s; cfl_hash_64bits_reset(&s);
  cfl_hash_64bits_update(&s, opts->fqname, cfl_sds_len(opts->fqname));
  for (int i = 0; i < labels_count; i++) { char *ptr = labels_val[i];
    unsigned len = strlen(ptr); cfl_hash_64bits_update(&s, ptr, len); }
  unsigned long hash = cfl_hash_64bits_digest(&s);
  struct metric *metric = metric_hash_lookup(map, hash, labels_count, labels_val);
  if (metric) return metric_prepare_storage(map, metric, write_op);
  metric = map_metric_create(map, hash, labels_count, labels_val);
  add(&metric->head, &map->metrics); metric_index_add(map, metric);
  map->last_metric = metric; return metric_prepare_storage(map, metric, write_op);
}
void cmt_map_metric_destroy(struct metric *metric) {}
void cmt_map_destroy(struct map *map) {
  cmt_map_metric_destroy(map->metrics);
  if (map->metric_static_set) { struct metric *metric = &map->metric; metric_release_storage(metric); }
  free(map);
}
struct metric *cmt_map_metric_get(struct opts *opts, struct map *map, int n, char **v, int write) {
  struct metric *metric; map_lock(map);
  metric = map_metric_get_unlocked(opts, map, n, v, write); map_unlock(map); return metric;
}
"#;

    #[test]
    fn pinned_hash_witness_is_zero() {
        assert_eq!(rrmxmx(ZERO_INPUT ^ ZERO_BITFLIP, 8), 0);
        assert_eq!(
            ZERO_INPUT.to_be_bytes(),
            [0xe1, 0x68, 0xd5, 0x42, 0xff, 0x27, 0xd7, 0x38]
        );
        assert!(model_reaches_violation());
        let prepared = OwnershipGraph {
            live: true,
            primary: vec![7],
            secondary: vec![7],
            cached: Some(7),
            ..OwnershipGraph::default()
        }
        .prepare(Selection::Embedded, true);
        assert_eq!(prepared.primary, [7]);
        assert_eq!(prepared.secondary, [7]);
        assert_eq!(prepared.cached, Some(7));
        assert!(prepared.borrow_valid(0, Selection::Embedded));
        let (destroyed, leaked) = prepared.destroy();
        assert!(leaked);
        assert!(!destroyed.borrow_valid(0, Selection::Embedded));
    }

    #[test]
    fn complete_graph_is_required_and_source_mutations_fail_closed() {
        let tokens = tokenize(GRAPH);
        assert!(extract(&tokens).is_some());
        assert!(
            extract(&tokenize(&GRAPH.replace("NULL", "((void*)0)"))).is_some(),
            "the pinned preprocessor's canonical NULL expansion is equivalent"
        );
        for (old, new) in [
            ("hash == 0", "hash == 1"),
            ("metric_static_set = 1", "metric_static_set = 0"),
            ("secret + 16", "secret + 17"),
            ("0x9FB21C651E98DF25ULL", "0x9FB21C651E98DF24ULL"),
            (
                "free(metric->hist_buckets)",
                "observe(metric->hist_buckets)",
            ),
            ("hist_buckets = NULL", "hist_buckets = ((void*)1)"),
            ("map_unlock(map)", "observe(map)"),
            ("cfl_sds_len(opts->fqname)", "8"),
            ("labels_count == 0", "labels_count == 1"),
            ("&s, ptr, len", "&s, ptr, 8"),
        ] {
            assert!(
                extract(&tokenize(&GRAPH.replace(old, new))).is_none(),
                "{old}"
            );
        }
    }

    #[test]
    fn complete_graph_adds_a_violation_without_erasing_existing_diagnostics() {
        let summary = analyze_legacy_c_translation_unit(
            GRAPH,
            "cmt_map_metric_get",
            &ResourceContracts::default(),
        )
        .expect("the reduced source graph analyses");
        let root = &summary.functions["cmt_map_metric_get"];
        assert_eq!(root.status, AnalysisStatus::Unsafe);
        assert!(
            root.diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. }))
        );
        assert!(root.diagnostics.len() > 1);
    }
}
