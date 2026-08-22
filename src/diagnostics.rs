use crate::Matches;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::{Display, Formatter};

const HISTOGRAM_BUCKETS: usize = 20;

#[derive(Debug)]
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
    // Convergence
    pub cap_relaxed: bool,
    /// Exact count of people with each shortlist length. Index = length; len = `target_shortlist` + 1.
    pub shortlist_len_histogram: Vec<usize>,
    pub appearance_max: usize,
    pub appearance_stddev: f32,
    pub zero_appearance_participants: Vec<String>,
    // Quality — histogram of served scores, auto-ranged over the observed [min, max].
    pub histogram: [usize; HISTOGRAM_BUCKETS],
    pub histogram_range: Option<(f32, f32)>, // (min_served, max_served); None if no scores served
    pub rank_regret_mean: f32,
    pub rank_regret_p95: usize,
    pub mutual_rate: f32,
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
            cap_relaxed: false,
            shortlist_len_histogram: Vec::new(),
            appearance_max: 0,
            appearance_stddev: 0.0,
            zero_appearance_participants: vec![],
            histogram: [0; HISTOGRAM_BUCKETS],
            histogram_range: None,
            rank_regret_mean: 0.0,
            rank_regret_p95: 0,
            mutual_rate: 0.0,
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
        // CONVERGENCE: label col 20, value col 8
        const C_LW: usize = 20;
        const C_VW: usize = 8;
        // QUALITY: label col 18, value col 8
        const Q_LW: usize = 18;
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
        writeln!(f)?;
        writeln!(
            f,
            "  pairs_scored: (male, female) pairs that survived all dealbreakers and got a score"
        )?;
        writeln!(
            f,
            "  dealbreaker_eliminated: pairs rejected before scoring due to dealbreakers"
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

#[allow(clippy::too_many_lines)]
pub fn build_diagnostics(
    pairs_stats: Option<PairsStats>,
    shortlist_stats: Option<ShortlistStats>,
    ids: &[&str],
    pairs: &FxHashMap<(&str, &str), f32>,
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
    let total_entries: usize = result.cards.iter().map(|c| c.shortlist.len()).sum();
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
    let mutual_rate = if total_entries == 0 {
        0.0
    } else {
        mutual_entries as f32 / total_entries as f32
    };

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
        cap_relaxed: ss.cap_relaxed,
        shortlist_len_histogram,
        appearance_max,
        appearance_stddev,
        zero_appearance_participants,
        histogram,
        histogram_range,
        rank_regret_mean,
        rank_regret_p95,
        mutual_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET_SHORTLIST: usize = 10;

    fn csv_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/many_generated.csv")
    }

    fn run_pipeline(collect_diagnostics: bool, seed: Option<u64>) -> (usize, Option<Diagnostics>) {
        let mut reader = csv::Reader::from_path(csv_path()).expect("test CSV should exist");
        let responses =
            crate::parsing::parse_responses(&mut reader).expect("test CSV should parse");
        let (_matches, diagnostics) = crate::matching::create_matches(
            &responses,
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

        let text = diags.to_string();
        assert!(!text.is_empty());
        assert!(text.contains("CONVERGENCE"));
    }

    #[test]
    fn smoke_diagnostics_none_when_disabled() {
        let (_, diagnostics) = run_pipeline(false, None);
        assert!(diagnostics.is_none());
    }
}
