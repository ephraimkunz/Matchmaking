use crate::matching::{Matches, PairScore, RankedCandidates};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use std::fmt::{Display, Formatter};

const HISTOGRAM_BUCKETS: usize = 20;

#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub seed: u64,
    // Pool
    pub male_count: usize,
    pub female_count: usize,
    pub pairs_scored: usize,
    pub dealbreaker_eliminated: usize,
    pub dealbreaker_by_wants_children: usize,
    pub dealbreaker_by_stay_local: usize,
    pub dealbreaker_by_marriage_timeline: usize,
    pub dealbreaker_by_religion: usize,
    /// Fraction of pair-score variance explained by additive per-person effects (some
    /// people score well, or poorly, with almost everyone) rather than genuine
    /// pair-specific compatibility. A property of the raw scoring, not of any one run.
    pub person_effect_share: f32,
    /// The most different subjects' top-`target_shortlist` *candidate* lists (by rank
    /// score) any single person appears on.
    pub demand_max: usize,
    /// People who appear on nobody's top-`target_shortlist` candidate list.
    pub demand_zero: usize,
    /// Gini coefficient of "how many different top-`target_shortlist` candidate lists
    /// each person appears on". 0 = every person is equally in-demand.
    pub demand_gini: f32,
    // Convergence
    pub cap_relaxed: bool,
    /// Exact count of people with each shortlist length. Index = length; len = `target_shortlist` + 1.
    pub shortlist_len_histogram: Vec<usize>,
    pub appearance_max: usize,
    pub appearance_stddev: f32,
    pub zero_appearance_participants: Vec<String>,
    /// Sum over everyone of `min(target_shortlist, viable_candidate_count)`: the most
    /// entries the pool could possibly support, regardless of algorithm quality.
    pub max_possible_entries: usize,
    pub entries_served: usize,
    /// People with a short shortlist because their pool of viable candidates was itself
    /// smaller than `target_shortlist` — the algorithm gave them everything available.
    pub pool_limited_short: usize,
    /// People with a short shortlist despite having enough viable candidates to fill
    /// it — the appearance cap, not the pool, is the reason. With a generous cap this
    /// should be 0; a tight cap can legitimately leave someone's one remaining
    /// candidate saturated by other people's picks, with no room left to raise the cap
    /// (`max_appearances_relaxed` already reached). Nonzero doesn't by itself mean a
    /// bug — check `cap_relaxed` and `appearance_max` alongside it before assuming one.
    pub algorithm_limited_short: usize,
    // Quality — histogram of served scores, auto-ranged over the observed [min, max].
    pub histogram: [usize; HISTOGRAM_BUCKETS],
    pub histogram_range: Option<(f32, f32)>, // (min_served, max_served); None if no scores served
    pub rank_regret_mean: f32,
    pub rank_regret_p95: usize,
    pub mutual_rate: f32,
    /// Total served score divided by the sum, over each person, of their own best
    /// `min(target_shortlist, viable_candidate_count)` display scores. 1.0 means the
    /// assignment captured every point of quality the pool allowed; anything lost is
    /// the cap/round-robin's fault, not the pool's.
    pub headroom_ratio: f32,
    pub headroom_worst_person: f32,
    pub headroom_p5: f32,
    /// Standard deviation of display scores across every scored pair.
    pub pair_score_stddev: f32,
    /// Mean, over people with at least 2 viable candidates, of (best display score −
    /// `target_shortlist`-th best display score, or worst if they have fewer).
    pub top_gap_mean: f32,
    /// `top_gap_mean` expressed in units of `pair_score_stddev`. Below roughly 1, the
    /// ranking near the top of a shortlist is weak signal, and shuffling among the top
    /// candidates (the default, unless `--sort-shortlists-by-score` is passed) is honest.
    pub top_gap_in_sds: f32,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            seed: 0,
            male_count: 0,
            female_count: 0,
            pairs_scored: 0,
            dealbreaker_eliminated: 0,
            dealbreaker_by_wants_children: 0,
            dealbreaker_by_stay_local: 0,
            dealbreaker_by_marriage_timeline: 0,
            dealbreaker_by_religion: 0,
            person_effect_share: 0.0,
            demand_max: 0,
            demand_zero: 0,
            demand_gini: 0.0,
            cap_relaxed: false,
            shortlist_len_histogram: Vec::new(),
            appearance_max: 0,
            appearance_stddev: 0.0,
            zero_appearance_participants: vec![],
            max_possible_entries: 0,
            entries_served: 0,
            pool_limited_short: 0,
            algorithm_limited_short: 0,
            histogram: [0; HISTOGRAM_BUCKETS],
            histogram_range: None,
            rank_regret_mean: 0.0,
            rank_regret_p95: 0,
            mutual_rate: 0.0,
            headroom_ratio: 1.0,
            headroom_worst_person: 1.0,
            headroom_p5: 1.0,
            pair_score_stddev: 0.0,
            top_gap_mean: 0.0,
            top_gap_in_sds: 0.0,
        }
    }
}

impl Display for Diagnostics {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const BAR_WIDTH: usize = 50;
        const HIST_LABEL_W: usize = 15; // wide enough for "0 (no matches)"
        // POOL: label col 24, value col 6; sub-item label col 22
        const P_LW: usize = 24;
        const P_VW: usize = 6;
        const P_SW: usize = 22;
        // CONVERGENCE: label col 22, value col 8
        const C_LW: usize = 22;
        const C_VW: usize = 8;
        // QUALITY: label col 20, value col 8
        const Q_LW: usize = 20;
        const Q_VW: usize = 8;
        writeln!(f, "Random seed used for generation: {}", self.seed)?;
        writeln!(f, "POOL  is the input usable?")?;
        writeln!(f, "  {:<P_LW$}{:>P_VW$}", "male_count", self.male_count)?;
        writeln!(f, "  {:<P_LW$}{:>P_VW$}", "female_count", self.female_count)?;
        writeln!(f, "  {:<P_LW$}{:>P_VW$}", "pairs_scored", self.pairs_scored)?;
        writeln!(
            f,
            "  {:<P_LW$}{:>P_VW$}",
            "dealbreaker_eliminated", self.dealbreaker_eliminated
        )?;
        writeln!(
            f,
            "    {:<P_SW$}{:>P_VW$}",
            "wants_children", self.dealbreaker_by_wants_children
        )?;
        writeln!(
            f,
            "    {:<P_SW$}{:>P_VW$}",
            "stay_local", self.dealbreaker_by_stay_local
        )?;
        writeln!(
            f,
            "    {:<P_SW$}{:>P_VW$}",
            "marriage_timeline", self.dealbreaker_by_marriage_timeline
        )?;
        writeln!(
            f,
            "    {:<P_SW$}{:>P_VW$}",
            "religion", self.dealbreaker_by_religion
        )?;
        writeln!(
            f,
            "  {:<P_LW$}{:>P_VW$}",
            "person_effect_share",
            format!("{:.1}%", self.person_effect_share * 100.0)
        )?;
        writeln!(f, "  {:<P_LW$}{:>P_VW$}", "demand_max", self.demand_max)?;
        writeln!(f, "  {:<P_LW$}{:>P_VW$}", "demand_zero", self.demand_zero)?;
        writeln!(
            f,
            "  {:<P_LW$}{:>P_VW$}",
            "demand_gini",
            format!("{:.2}", self.demand_gini)
        )?;
        writeln!(f)?;
        writeln!(
            f,
            "  pairs_scored: (male, female) pairs that survived all dealbreakers and got a score"
        )?;
        writeln!(
            f,
            "  dealbreaker_eliminated: pairs rejected before scoring due to dealbreakers"
        )?;
        writeln!(
            f,
            "  person_effect_share: how much of the score spread is \"this person rates/is rated highly by everyone\" rather than genuine pair fit. High means the scoring is closer to a popularity contest than a compatibility measure."
        )?;
        writeln!(
            f,
            "  demand_max: the most different people who put the same person in their top-{{target_shortlist}} candidate list"
        )?;
        writeln!(
            f,
            "  demand_zero: people nobody's top-{{target_shortlist}} candidate list includes, before the appearance cap or assignment even runs"
        )?;
        writeln!(
            f,
            "  demand_gini: inequality of that same demand count across everyone; 0 = perfectly even, closer to 1 = a few people are everyone's favorite"
        )?;

        // ── CONVERGENCE ───────────────────────────────────────────────────────
        writeln!(f, "\nCONVERGENCE  did the algorithm finish cleanly?")?;
        writeln!(f, "  {:<C_LW$}{:>C_VW$}", "cap_relaxed", self.cap_relaxed)?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "appearance_max", self.appearance_max
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "appearance_stddev",
            format!("{:.2}", self.appearance_stddev)
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}: {:?}",
            "zero_appearances",
            self.zero_appearance_participants.len(),
            self.zero_appearance_participants
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "possible_entries", self.max_possible_entries
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "entries_served", self.entries_served
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "pool_limited_short", self.pool_limited_short
        )?;
        writeln!(
            f,
            "  {:<C_LW$}{:>C_VW$}",
            "algo_limited_short", self.algorithm_limited_short
        )?;
        writeln!(f)?;
        writeln!(f, "  shortlist lengths")?;
        let max_len_count = *self.shortlist_len_histogram.iter().max().unwrap_or(&1);
        let target_len = self.shortlist_len_histogram.len().saturating_sub(1);
        for (length, &count) in self.shortlist_len_histogram.iter().enumerate() {
            let label = if length == 0 {
                format!("{length} (no matches)")
            } else if length == target_len {
                format!("{length} (full)")
            } else {
                format!("{length}")
            };
            let bar_len = (count * BAR_WIDTH).checked_div(max_len_count).unwrap_or(0);
            let bar: String = "#".repeat(bar_len);
            writeln!(f, "    {label:<HIST_LABEL_W$} | {bar:<BAR_WIDTH$}  {count}")?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "  cap_relaxed: true if the appearance cap had to be raised to make progress; true = pool was tight and quality may have suffered"
        )?;
        writeln!(
            f,
            "  appearance_max: the most times any one person was picked; should sit at the cap when the pool is tight"
        )?;
        writeln!(
            f,
            "  appearance_stddev: spread of pick counts; low = even distribution, high = a few popular people absorbed many picks while others got none"
        )?;
        writeln!(
            f,
            "  zero_appearances: people no one's shortlist included; see histogram index 0 for the subject-side complement"
        )?;
        writeln!(
            f,
            "  possible_entries: the most shortlist entries the pool could support even with a perfect algorithm and no appearance cap"
        )?;
        writeln!(
            f,
            "  entries_served: shortlist entries actually produced; possible_entries - entries_served is capacity the run left unused"
        )?;
        writeln!(
            f,
            "  pool_limited_short: people whose shortlist is short only because they don't have enough viable candidates; not fixable by tuning the algorithm"
        )?;
        writeln!(
            f,
            "  algo_limited_short: people whose shortlist is short despite having enough viable candidates; the reason is the appearance cap, not the pool. Zero with a generous cap; a nonzero count under a tight cap can be an honest capacity trade-off, not necessarily a bug — cross-check cap_relaxed and appearance_max, and consider raising --max-appearances-relaxed"
        )?;
        writeln!(
            f,
            "  shortlist lengths: exact count of people with each shortlist length; 0 = no matches, last bucket = full target"
        )?;

        // ── QUALITY ───────────────────────────────────────────────────────────
        writeln!(f, "\nQUALITY  is the output good?")?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "rank_regret_mean",
            format!("{:.2}", self.rank_regret_mean)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "rank_regret_p95", self.rank_regret_p95
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "mutual_rate",
            format!("{:.1}%", self.mutual_rate * 100.0)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "headroom_ratio",
            format!("{:.1}%", self.headroom_ratio * 100.0)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "headroom_worst",
            format!("{:.1}%", self.headroom_worst_person * 100.0)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "headroom_p5",
            format!("{:.1}%", self.headroom_p5 * 100.0)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "pair_score_stddev",
            format!("{:.3}", self.pair_score_stddev)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "top_gap_mean",
            format!("{:.3}", self.top_gap_mean)
        )?;
        writeln!(
            f,
            "  {:<Q_LW$}{:>Q_VW$}",
            "top_gap_in_sds",
            format!("{:.2}", self.top_gap_in_sds)
        )?;
        writeln!(f)?;
        writeln!(f, "  score distribution")?;
        match self.histogram_range {
            None => writeln!(f, "    (no scores served)")?,
            Some((lo, hi)) => {
                let width = (hi - lo) / HISTOGRAM_BUCKETS as f32;
                let max_count = *self.histogram.iter().max().unwrap_or(&1);
                for (i, &count) in self.histogram.iter().enumerate() {
                    let bucket_lo = lo + i as f32 * width;
                    let bucket_hi = if i == HISTOGRAM_BUCKETS - 1 {
                        hi
                    } else {
                        bucket_lo + width
                    };
                    let label = format!("{bucket_lo:.3}-{bucket_hi:.3}");
                    let bar_len = (count * BAR_WIDTH).checked_div(max_count).unwrap_or(0);

                    let bar: String = "#".repeat(bar_len);
                    writeln!(f, "    {label:<HIST_LABEL_W$} | {bar:<BAR_WIDTH$}  {count}")?;
                }
            }
        }
        writeln!(f)?;
        writeln!(
            f,
            "  rank_regret_mean: extra candidates skipped per pick because higher-ranked options were at the appearance cap. 0 = every pick was the best still-available match; 2 = on average the cap forced 2 better candidates to be skipped before each pick. Larger means the cap is biting harder."
        )?;
        writeln!(
            f,
            "  rank_regret_p95: same skip-count, 95th percentile. A small mean with a large p95 means most picks were unblocked but a few people had popular candidates capped out and got pushed deep into their list."
        )?;
        writeln!(
            f,
            "  mutual_rate: fraction of shortlist entries where B is also on A's list; 100% = every match is mutual, low values mean many one-sided introductions"
        )?;
        writeln!(
            f,
            "  headroom_ratio: served score as a fraction of the best score the pool could have given everyone if the appearance cap and round-robin were perfect. Low values mean the assignment, not the pool, is costing quality — the fix is a better algorithm, not better data."
        )?;
        writeln!(
            f,
            "  headroom_worst / headroom_p5: same ratio for the single worst-served person, and the 5th percentile; a healthy run keeps these close to headroom_ratio"
        )?;
        writeln!(
            f,
            "  pair_score_stddev: spread of display scores across every scored pair; the yardstick the two gaps below are measured against"
        )?;
        writeln!(
            f,
            "  top_gap_mean: average gap between a person's best and their target_shortlist-th best candidate; small means the top of everyone's list is nearly tied"
        )?;
        writeln!(
            f,
            "  top_gap_in_sds: top_gap_mean divided by pair_score_stddev; below about 1, ranking within a shortlist is noise, and default random shortlist order is the honest choice"
        )?;
        writeln!(
            f,
            "  score distribution: distribution of scores that were actually served, auto-ranged to the observed [min, max]; mass in high buckets is healthy, weight in low buckets means someone got a poor match"
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DealbreakerCause {
    WantsChildren,
    StayLocal,
    MarriageTimeline,
    Religion,
}

pub struct PairsStats {
    pub male_count: usize,
    pub female_count: usize,
    pub dealbreaker_eliminated: usize,
    pub dealbreaker_by_wants_children: usize,
    pub dealbreaker_by_stay_local: usize,
    pub dealbreaker_by_marriage_timeline: usize,
    pub dealbreaker_by_religion: usize,
}

pub struct ShortlistStats {
    pub cap_relaxed: bool,
    pub appearance_count: FxHashMap<String, usize>,
    pub served_excess_ranks: Vec<usize>,
}

/// Population variance (divide by n, not n-1) of a slice of scores. Matches the
/// convention already used for `appearance_stddev`.
fn population_variance(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32
}

/// Gini coefficient of a non-negative distribution via the sorted-cumulative form
/// `G = 2*sum(i * x_i) / (n * sum(x_i)) - (n + 1) / n` (1-indexed, ascending). 0 = every
/// value equal; approaching 1 = concentrated in a few values. Used on "how many
/// different people's top-N candidate list this person appears in".
fn gini_coefficient(counts: &[usize]) -> f32 {
    let n = counts.len();
    let total: usize = counts.iter().sum();
    if n == 0 || total == 0 {
        return 0.0;
    }
    let mut sorted = counts.to_vec();
    sorted.sort_unstable();
    let weighted_sum: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &c)| (i + 1) as f64 * c as f64)
        .sum();
    let n = n as f64;
    let total = total as f64;
    // Safe: the Gini coefficient of a non-negative distribution is always in [0, 1],
    // well within f32 precision.
    #[allow(clippy::cast_possible_truncation)]
    let gini = ((2.0 * weighted_sum) / (n * total) - (n + 1.0) / n) as f32;
    gini
}

/// Fit `display_score(a, b) ≈ mu + alpha_a + alpha_b` by alternating least squares, and
/// return the fraction of total score variance explained by the additive per-person
/// effects, as opposed to genuine pair-specific compatibility. This is a property of the
/// raw scoring function on this input, independent of any calibration the matching step
/// applies downstream.
fn fit_person_effect_share(pairs: &FxHashMap<(&str, &str), PairScore>) -> f32 {
    const ITERATIONS: usize = 25;

    if pairs.len() < 2 {
        return 0.0;
    }

    let scores: Vec<f32> = pairs.values().map(|p| p.display).collect();
    let mu = scores.iter().sum::<f32>() / scores.len() as f32;
    let total_variance = population_variance(&scores);
    if total_variance <= f32::EPSILON {
        return 0.0;
    }

    let mut incident: FxHashMap<&str, Vec<(&str, f32)>> = FxHashMap::default();
    for (&(a, b), score) in pairs {
        incident.entry(a).or_default().push((b, score.display));
        incident.entry(b).or_default().push((a, score.display));
    }

    let mut alpha: FxHashMap<&str, f32> = incident.keys().map(|&k| (k, 0.0)).collect();
    for _ in 0..ITERATIONS {
        let mut next_alpha: FxHashMap<&str, f32> = FxHashMap::default();
        for (&person, neighbors) in &incident {
            let sum: f32 = neighbors
                .iter()
                .map(|&(other, score)| score - mu - alpha[other])
                .sum();
            next_alpha.insert(person, sum / neighbors.len() as f32);
        }
        // Re-center to mean zero each pass so `mu` stays the grand mean.
        let mean_alpha = next_alpha.values().sum::<f32>() / next_alpha.len() as f32;
        for v in next_alpha.values_mut() {
            *v -= mean_alpha;
        }
        alpha = next_alpha;
    }

    let residuals: Vec<f32> = pairs
        .iter()
        .map(|(&(a, b), score)| score.display - mu - alpha[a] - alpha[b])
        .collect();
    let residual_variance = population_variance(&residuals);

    (1.0 - residual_variance / total_variance).clamp(0.0, 1.0)
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn build_diagnostics(
    pairs_stats: Option<PairsStats>,
    shortlist_stats: Option<ShortlistStats>,
    ids: &[&str],
    pairs: &FxHashMap<(&str, &str), PairScore>,
    ranked_candidates: &RankedCandidates,
    result: &Matches,
    target_shortlist: usize,
    seed: u64,
) -> Option<Diagnostics> {
    let (ps, ss) = pairs_stats.zip(shortlist_stats)?;
    let total_participants = ids.len();

    // Convergence: exact histogram of shortlist lengths (index = length, value = # people).
    // Lengths are bounded to [0, target_shortlist] since the loop never over-fills.
    let mut shortlist_len_histogram = vec![0usize; target_shortlist + 1];
    for card in &result.cards {
        let len = card.shortlist.len().min(target_shortlist);
        shortlist_len_histogram[len] += 1;
    }

    // Appearance stats
    let appearance_max = ss.appearance_count.values().copied().max().unwrap_or(0);
    let zero_appearance_participants = ids
        .iter()
        .filter_map(|id| {
            if ss.appearance_count.get(*id).copied().unwrap_or(0) == 0 {
                Some(id.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let counts_with_zeros: Vec<f32> = ids
        .iter()
        .map(|id| ss.appearance_count.get(*id).copied().unwrap_or(0) as f32)
        .collect();
    let mean_appearances = counts_with_zeros.iter().sum::<f32>() / total_participants.max(1) as f32;
    let variance = counts_with_zeros
        .iter()
        .map(|&c| (c - mean_appearances).powi(2))
        .sum::<f32>()
        / total_participants.max(1) as f32;
    let appearance_stddev = variance.sqrt();

    // Quality: histogram of served scores, auto-ranged over the observed [min, max] so the
    // chart keeps full resolution no matter where the scores cluster.
    let mut histogram = [0usize; HISTOGRAM_BUCKETS];
    let served_scores: Vec<f32> = result
        .cards
        .iter()
        .flat_map(|c| c.shortlist.iter().map(|m| m.score))
        .collect();
    let entries_served = served_scores.len();
    let histogram_range = served_scores
        .iter()
        .copied()
        .fold(None, |acc: Option<(f32, f32)>, s| match acc {
            None => Some((s, s)),
            Some((lo, hi)) => Some((lo.min(s), hi.max(s))),
        });
    if let Some((lo, hi)) = histogram_range {
        let range = hi - lo;
        for score in &served_scores {
            let idx = if range <= f32::EPSILON {
                0
            } else {
                // Safe: (score-lo)/range ∈ [0,1] so the product ∈ [0, HISTOGRAM_BUCKETS]
                // and floor() ≥ 0, so truncation and sign-loss are impossible in practice.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let v = (((score - lo) / range) * HISTOGRAM_BUCKETS as f32).floor() as usize;
                v
            }
            .min(HISTOGRAM_BUCKETS - 1);
            histogram[idx] += 1;
        }
    }

    // Feasibility: how much of the shortfall is the pool's fault vs. the algorithm's.
    let mut max_possible_entries = 0usize;
    let mut pool_limited_short = 0usize;
    let mut algorithm_limited_short = 0usize;
    for card in &result.cards {
        let candidate_count = ranked_candidates
            .get(card.email.as_str())
            .map_or(0, Vec::len);
        max_possible_entries += candidate_count.min(target_shortlist);
        let served = card.shortlist.len();
        if served < target_shortlist {
            if candidate_count <= served {
                pool_limited_short += 1;
            } else {
                algorithm_limited_short += 1;
            }
        }
    }

    // Rank-regret: for each served pick, how many candidates were skipped because
    // they were at the appearance cap (excess over the minimum possible rank).
    let mut excess_ranks: Vec<usize> = ss.served_excess_ranks.clone();
    let rank_regret_mean = if excess_ranks.is_empty() {
        0.0
    } else {
        excess_ranks.iter().sum::<usize>() as f32 / excess_ranks.len() as f32
    };
    let rank_regret_p95 = if excess_ranks.is_empty() {
        0
    } else {
        let p95_idx = (excess_ranks.len() * 95)
            .div_ceil(100)
            .saturating_sub(1)
            .min(excess_ranks.len() - 1);
        excess_ranks.select_nth_unstable(p95_idx);
        excess_ranks[p95_idx]
    };

    // Mutuality: for every (A -> B) entry in all shortlists, check whether B's shortlist
    // also contains A.
    let shortlist_index: FxHashMap<&str, FxHashSet<&str>> = result
        .cards
        .iter()
        .map(|card| {
            let members: FxHashSet<&str> =
                card.shortlist.iter().map(|m| m.email.as_str()).collect();
            (card.email.as_str(), members)
        })
        .collect();
    let mutual_entries: usize = result
        .cards
        .iter()
        .flat_map(|card| {
            card.shortlist.iter().map(|m| {
                usize::from(
                    shortlist_index
                        .get(m.email.as_str())
                        .is_some_and(|their_list| their_list.contains(card.email.as_str())),
                )
            })
        })
        .sum();
    let mutual_rate = if entries_served == 0 {
        0.0
    } else {
        mutual_entries as f32 / entries_served as f32
    };

    // Each person's own candidate display scores, sorted descending. Used by both the
    // headroom and separability metrics below — distinct from `ranked_candidates`'
    // own order, which is sorted by (calibrated) rank score, not display score.
    let sorted_display: FxHashMap<&str, Vec<f32>> = ids
        .iter()
        .map(|&id| {
            let mut scores: Vec<f32> = ranked_candidates
                .get(id)
                .map(|v| v.iter().map(|&(_, _, display)| display).collect())
                .unwrap_or_default();
            scores.sort_unstable_by(|a, b| {
                b.partial_cmp(a).expect("Should be able to compare floats")
            });
            (id, scores)
        })
        .collect();

    // Headroom: served score vs. each person's own best-possible score at this target,
    // ignoring the appearance cap entirely.
    let mut served_total = 0.0f32;
    let mut ideal_total = 0.0f32;
    let mut headroom_ratios: Vec<f32> = Vec::with_capacity(total_participants);
    for card in &result.cards {
        let served_sum: f32 = card.shortlist.iter().map(|m| m.score).sum();
        let ideal_sum: f32 = sorted_display
            .get(card.email.as_str())
            .map_or(0.0, |scores| scores.iter().take(target_shortlist).sum());
        served_total += served_sum;
        ideal_total += ideal_sum;
        headroom_ratios.push(if ideal_sum > f32::EPSILON {
            served_sum / ideal_sum
        } else {
            1.0
        });
    }
    let headroom_ratio = if ideal_total > f32::EPSILON {
        served_total / ideal_total
    } else {
        1.0
    };
    headroom_ratios.sort_unstable_by(|a, b| a.partial_cmp(b).expect("Should compare floats"));
    let headroom_worst_person = headroom_ratios.first().copied().unwrap_or(1.0);
    let headroom_p5 = if headroom_ratios.is_empty() {
        1.0
    } else {
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation
        )]
        let idx = ((headroom_ratios.len() as f32) * 0.05).floor() as usize;
        headroom_ratios[idx.min(headroom_ratios.len() - 1)]
    };

    // Separability: how much the top of a shortlist actually stands out from the rest,
    // in units of the overall spread of display scores.
    let pair_score_stddev =
        population_variance(&pairs.values().map(|p| p.display).collect::<Vec<_>>()).sqrt();
    let gaps: Vec<f32> = sorted_display
        .values()
        .filter(|scores| scores.len() >= 2)
        .map(|scores| {
            let last_idx = target_shortlist.saturating_sub(1).min(scores.len() - 1);
            scores[0] - scores[last_idx]
        })
        .collect();
    let top_gap_mean = if gaps.is_empty() {
        0.0
    } else {
        gaps.iter().sum::<f32>() / gaps.len() as f32
    };
    let top_gap_in_sds = if pair_score_stddev > f32::EPSILON {
        top_gap_mean / pair_score_stddev
    } else {
        0.0
    };

    // Person effects: a property of the raw scoring, computed on display scores only.
    let person_effect_share = fit_person_effect_share(pairs);

    // Demand concentration: how many different subjects put each candidate in their own
    // top-target_shortlist, by the same (calibrated) rank order the assignment uses.
    let mut demand_count: FxHashMap<&str, usize> = ids.iter().map(|&id| (id, 0)).collect();
    for candidates in ranked_candidates.values() {
        for &(candidate_id, _rank_score, _display_score) in candidates.iter().take(target_shortlist)
        {
            *demand_count.entry(candidate_id).or_insert(0) += 1;
        }
    }
    let demand_counts: Vec<usize> = ids
        .iter()
        .map(|id| demand_count.get(*id).copied().unwrap_or(0))
        .collect();
    let demand_max = demand_counts.iter().copied().max().unwrap_or(0);
    let demand_zero = demand_counts.iter().filter(|&&c| c == 0).count();
    let demand_gini = gini_coefficient(&demand_counts);

    Some(Diagnostics {
        seed,
        male_count: ps.male_count,
        female_count: ps.female_count,
        pairs_scored: pairs.len(),
        dealbreaker_eliminated: ps.dealbreaker_eliminated,
        dealbreaker_by_wants_children: ps.dealbreaker_by_wants_children,
        dealbreaker_by_stay_local: ps.dealbreaker_by_stay_local,
        dealbreaker_by_marriage_timeline: ps.dealbreaker_by_marriage_timeline,
        dealbreaker_by_religion: ps.dealbreaker_by_religion,
        person_effect_share,
        demand_max,
        demand_zero,
        demand_gini,
        cap_relaxed: ss.cap_relaxed,
        shortlist_len_histogram,
        appearance_max,
        appearance_stddev,
        zero_appearance_participants,
        max_possible_entries,
        entries_served,
        pool_limited_short,
        algorithm_limited_short,
        histogram,
        histogram_range,
        rank_regret_mean,
        rank_regret_p95,
        mutual_rate,
        headroom_ratio,
        headroom_worst_person,
        headroom_p5,
        pair_score_stddev,
        top_gap_mean,
        top_gap_in_sds,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // exact comparisons against deterministic, hand-computed values
mod tests {
    use super::*;
    use crate::rng_and_seed;

    const TARGET_SHORTLIST: usize = 10;

    fn csv_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/many_generated.csv")
    }

    fn run_pipeline(collect_diagnostics: bool, seed: Option<u64>) -> (usize, Option<Diagnostics>) {
        let mut reader = csv::Reader::from_path(csv_path()).expect("test CSV should exist");
        let (mut rng, _) = rng_and_seed(None);
        let responses =
            crate::parsing::parse_responses(&mut reader, &mut rng).expect("test CSV should parse");
        let (mut rng, seed) = rng_and_seed(seed);
        let (_matches, diagnostics) = crate::matching::create_matches(
            &responses,
            &mut rng,
            seed,
            false,
            false,
            collect_diagnostics,
            TARGET_SHORTLIST,
            TARGET_SHORTLIST,
            TARGET_SHORTLIST * 2 + 5,
            None,
        );
        (responses.len(), diagnostics)
    }

    #[test]
    fn smoke_display_renders_default_diagnostics() {
        let text = Diagnostics::default().to_string();
        assert!(!text.is_empty());
        assert!(text.contains("CONVERGENCE"));
    }

    #[test]
    fn smoke_diagnostics_invariants_on_real_data() {
        let (total, diagnostics) = run_pipeline(true, Some(42));
        let diags = diagnostics.expect("diagnostics should be collected when enabled");

        assert_eq!(diags.seed, 42);
        assert!(total > 0);
        assert_eq!(diags.male_count + diags.female_count, total);
        assert_eq!(diags.shortlist_len_histogram.len(), TARGET_SHORTLIST + 1);
        assert_eq!(diags.shortlist_len_histogram.iter().sum::<usize>(), total);
        assert_eq!(
            diags.dealbreaker_by_wants_children
                + diags.dealbreaker_by_stay_local
                + diags.dealbreaker_by_marriage_timeline
                + diags.dealbreaker_by_religion,
            diags.dealbreaker_eliminated
        );
        assert!((0.0..=1.0).contains(&diags.mutual_rate));
        assert!((0.0..=1.0).contains(&diags.person_effect_share));
        assert!((0.0..=1.0).contains(&diags.demand_gini));
        assert!(diags.demand_max <= total);
        assert!(diags.entries_served <= diags.max_possible_entries);
        // `run_pipeline` uses a deliberately loose cap (max_appearances ==
        // target_shortlist, relaxed to 2x+5), so the appearance cap ceiling should
        // never bind here and everyone gets everything their own pool allows. This is
        // not a universal invariant: with the CLI's tighter defaults, a saturated
        // candidate can legitimately leave this nonzero — see the field's doc comment.
        assert_eq!(diags.algorithm_limited_short, 0);
        assert_eq!(diags.entries_served, diags.max_possible_entries);
        assert!(diags.headroom_ratio >= 0.0 && diags.headroom_ratio <= 1.0 + 1e-4);
        assert!(diags.pair_score_stddev >= 0.0);

        let text = diags.to_string();
        assert!(!text.is_empty());
        assert!(text.contains("CONVERGENCE"));
        assert!(text.contains("headroom_ratio"));
        assert!(text.contains("demand_gini"));
        assert!(text.contains("possible_entries"));
    }

    #[test]
    fn smoke_diagnostics_none_when_disabled() {
        let (_, diagnostics) = run_pipeline(false, None);
        assert!(diagnostics.is_none());
    }

    #[test]
    fn smoke_diagnostics_json_round_trips() {
        let (_, diagnostics) = run_pipeline(true, Some(7));
        let diags = diagnostics.expect("diagnostics should be collected when enabled");
        let json = serde_json::to_string(&diags).expect("Diagnostics should serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("serialized diagnostics should parse as JSON");
        assert_eq!(value["seed"], 7);
        assert!(value["headroom_ratio"].is_number());
        assert!(value["demand_gini"].is_number());
    }

    #[test]
    fn gini_coefficient_is_zero_when_even_and_positive_when_concentrated() {
        assert_eq!(gini_coefficient(&[3, 3, 3, 3]), 0.0);
        assert_eq!(gini_coefficient(&[]), 0.0);
        assert_eq!(gini_coefficient(&[0, 0, 0]), 0.0);
        let uneven = gini_coefficient(&[0, 0, 0, 10]);
        assert!(uneven > 0.5, "expected high inequality, got {uneven}");
    }
}
