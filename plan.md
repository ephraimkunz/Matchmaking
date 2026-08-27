 Fix the scoring objective and add the metrics that show where headroom is

 Context

 The question was whether re-running the randomized shortlist assignment many times and
 keeping the best result (hill climbing) would produce better matches, and whether the
 current diagnostics are sufficient to judge "better".

 I measured it on the real data (~/Downloads/out.csv, 110 people, 56M/54F, -t 5 -a 8 -r 10):

 ┌─────────────────────────────────┬────────────────────────────────────────────────────────────────────────────┐
 │           Measurement           │                                   Result                                   │
 ├─────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
 │ 30 different seeds              │ total entries 542 in every run; mutual_rate 52.8–55.0%; rank_regret_mean   │
 │                                 │ 0.91–0.99; zero_appearances 8–10                                           │
 ├─────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
 │                                 │ 542 served = 542 possible (Σ min(target, viable_candidates)). All 4 short  │
 │ Coverage vs. theoretical max    │ lists are pool-limited: those people have exactly 1, 3, 4 and 4 viable     │
 │                                 │ candidates.                                                                │
 ├─────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
 │ Greedy total score vs. each     │ 99.42%                                                                     │
 │ person's unconstrained top-5    │                                                                            │
 ├─────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
 │ Provably optimal assignment at  │ 99.78%                                                                     │
 │ the same cap (min-cost flow)    │                                                                            │
 └─────────────────────────────────┴────────────────────────────────────────────────────────────────────────────┘

 So the entire headroom available to any smarter assignment — best-of-K restarts, local
 search, or an exact solve — is 0.36% of total score, about 0.003 per shortlist entry on
 a 0.44–0.83 scale. Hill climbing cannot move this. We are not building it.

 The headroom is upstream, in the score function, and there are two concrete problems:

 1. The cross-match operator is wrong for 6 of its 8 items. matching.rs:455-466 applies
 a bipolar operator 1 − |want − have| to all 8 cross-matched pairs. Checking
 questionnaire_structure.json, only 2 of the 8 preference questions are actually bipolar
 ("I'd prefer a partner who plans vs. goes with the flow" 1=Spontaneous…4=Planner; "I'd
 prefer a partner who is" 1=Homebody…4=Very social). The other 6 are importance scales
 ("An artistic or creative side in a partner matters" 1=Not at all…4=Very much). For those,
 answering "doesn't matter" currently penalizes you for a partner who has the trait.
 In the real data 33.2% of those term evaluations are wrongly under-scored, mean loss 0.475
 similarity, shifting a directional score by up to 0.039 — larger than the whole #1-to-#5
 gap (0.032) and comparable to the #1-to-#10 gap (0.051).

 2. Scores are not calibrated per person. Decomposing pair scores as
 score(a,b) ≈ μ + person_a + person_b + pair_specific: 58.6% of the variance is additive
 person effects (some people score high with everybody), only 41.4% is genuine pair
 compatibility. Consequences in the real run: 19 people appear in ≥10 different top-5
 candidate lists (max 24) while 19 people appear in nobody's top-5; the appearance cap
 has to relax; rank_regret_mean is 0.92 with p95 = 4. Mean-centering each person's
 directional scores before combining drops top-5 demand Gini 0.549 → 0.302, max demand
 24 → 12, and never-wanted people 19 → 2.

 Performance needs no work: 1000 people at --target-shortlist 20 runs in 40 ms.

 Intended outcome: the score ranks "who fits me" rather than "who is broadly agreeable", and
 the diagnostics report enough to answer "is the assignment or the scoring the bottleneck?"
 without re-deriving any of the above by hand. Shortlists will visibly change — that is the
 point, since the current lists reflect the bug.

 ---

 Phase 1 — Fix the cross-match operator

 src/matching.rs, process_self_and_partner (lines 421-469).

 Add a per-item table next to the existing section-weight constants. Pair k couples
 PartnerPreferences.crossmatched[k] with SelfDescription.crossmatched[k]; the index maps
 are parsing.rs:581 ([0,1,2,3,7,8,10,11]) and parsing.rs:610 ([0..7]), so:

 /// Which of the 8 cross-matched pairs use a bipolar preference scale, where the
 /// respondent picks a point on the trait axis, so `1 - |want - have|` is correct.
 /// The rest use an importance scale ("...in a partner matters", 1 = not at all),
 /// where wanting less of a trait must not penalize a partner who has more of it.
 const CROSSMATCH_IS_BIPOLAR: [bool; 8] = [
     true,  // 0 plans carefully        <-> prefers a planner vs. go-with-the-flow
     false, // 1 artistic side          <-> artistic side matters
     true,  // 2 energetic and outgoing <-> prefers homebody vs. very social
     false, // 3 goal-oriented          <-> ambition matters
     false, // 4 dry sense of humor     <-> dry humor matters
     false, // 5 enjoys intellectual debate <-> intellectually curious matters
     false, // 6 diet and nutrition     <-> health-conscious matters
     false, // 7 staying active         <-> active or fit matters
 ];

 In the cross-match loop, replace the unconditional 1.0 - diff with:

 let want = a_answer.normalized();
 let have = b_answer.normalized();
 let similarity = if CROSSMATCH_IS_BIPOLAR[k] {
     1.0 - f32::abs(want - have)
 } else {
     // Shortfall only: penalize a partner who has less of the trait than I asked for.
     // Indifference (want = 0.0) is always satisfied; excess is never punished.
     1.0 - f32::max(0.0, want - have)
 };

 Use .enumerate() on the existing zip to get k. Nothing else in that function changes.

 Phase 2 — Calibrate directional scores per person

 Rank by a calibrated score, but keep displaying the raw one, so the printed card score and
 the served-score histogram stay interpretable.

 src/matching.rs, build_scored_pairs (lines 529-583). Restructure into three passes:

 1. For every surviving (male, female) pair, compute and store both directions,
    raw_ab = directional_score(m, f) and raw_ba = directional_score(f, m).
    Hoist calculate_subject_chosen_weight_scale_factor out of directional_score and pass
    it in — it is a per-subject constant currently recomputed 2 × M × F times.
 2. For each person, take the mean of their own stored directional scores over their viable
    candidates. Guard the degenerate case: fewer than 2 candidates ⇒ mean 0.0 (calibration
    becomes a no-op for that person).
 3. Combine:
    - display = 0.8·min(raw_ab, raw_ba) + 0.2·midpoint(raw_ab, raw_ba) — unchanged from today.
    - rank = 0.8·min(cal_ab, cal_ba) + 0.2·midpoint(cal_ab, cal_ba) where
      cal_ab = raw_ab − mean_m, cal_ba = raw_ba − mean_f.

 Mean-centering alone is what produced the Gini 0.549 → 0.302 result; do not also divide by
 the standard deviation. rank is signed — that is fine, nothing displays it.

 Change the pair table from FxHashMap<(&str, &str), f32> to
 FxHashMap<(&str, &str), PairScore> with struct PairScore { display: f32, rank: f32 }.
 Sort ranked_candidates by .rank; store .display in the shortlist tuple that becomes
 ShortlistMatch::score.

 No CLI flag — calibration is unconditional, applied the same way mutual_score is today.
 --seed plus before/after diagnostics on the real data (see Verification) is the A/B.

 Phase 3 — Split the deterministic prep out of the randomized assignment

 Move the "build and sort ranked_candidates" block (matching.rs:648-684) out of
 assign_shortlists and into create_matches, then pass &ranked_candidates to both
 assign_shortlists and build_diagnostics.

 This is needed because the new metrics require the ranked lists, and it cleanly separates
 the expensive deterministic phase from the cheap random one. It is also exactly the split a
 restart loop would have needed, if a future pool ever turns out tight enough to justify one.

 While moving it, fix the panic at matching.rs:682: ranked_candidates[id.as_str()] aborts
 with no entry found for key on an unknown --debug-print-candidate-list email (verified,
 exit 134). Use .get(), write to stderr instead of stdout (it currently interleaves with
 plain-text match output), and print a clear "no participant with id ..." line when missing.

 Phase 4 — Add the four missing metrics

 src/diagnostics.rs. Add #[derive(Serialize)] to Diagnostics and a
 --diagnostics-format {text,json} flag in main.rs so this is scriptable. Follow the
 existing convention: each metric gets an inline prose interpretation line in the Display
 impl.

 A. Headroom — new QUALITY entries. Per person, served_display_sum / ideal_display_sum
 where ideal is their own top-min(target, |candidates|) display scores. Report
 headroom_ratio (overall), headroom_worst_person, headroom_p5. This is the metric that
 answers the original question directly: it read 0.994 here, so 0.6% was all any assignment
 change could recover.

 B. Feasibility — new CONVERGENCE entries. max_possible_entries = Σ min(target, |candidates|), entries_served, and split the short lists into pool_limited_short vs.
 algorithm_limited_short. algorithm_limited_short == 0 means the assignment did
 everything the pool allows — true in every run I measured, and it distinguishes "the
 algorithm failed" from "the pool made it impossible" (which cap_relaxed alone cannot:
 -t 12 -a 3 needs 12 000 picks against 4 000 capacity and reports only cap_relaxed=true).

 C. Separability — new QUALITY entries. pair_score_stddev over all scored display
 scores, top_gap_mean = mean over people of (rank-1 score − rank-target score), and
 top_gap_in_sds = the ratio. Below ~1 SD the ranking near the top of each list is weak
 signal and randomizing among the top is honest. It reads 0.032 raw / 0.70 SD here.

 D. Person effects and demand concentration — new POOL entries.
 person_effect_share: fit score(a,b) ≈ μ + α_a + α_b by alternating least squares (~20
 iterations, O(pairs) each, re-centering α to mean zero each pass) and report
 1 − var(residual)/var(scores). Plus, over "how many people's top-target candidate list
 does this person appear in" (candidate lists, not served shortlists, so it measures the
 scoring independently of the assignment): demand_max, demand_zero, and demand_gini
 via the sorted form G = 2·Σ(i·x_i)/(n·Σx_i) − (n+1)/n.

 Phase 5 — Small fixes found along the way

 - main.rs:95: Command::new("open") is macOS-only, but releases ship for Windows and
   Linux. Select the launcher per platform (open / cmd /C start / xdg-open) and do not
   fail the run when the launcher is absent — matches.docx is already written by then.
 - src/main.rs:1-7 holds the only crate-level clippy block, so matching.rs,
   parsing.rs, diagnostics.rs and docx.rs are not covered by clippy::pedantic even
   though they carry #[allow] suppressions written as if they were. Move the block to
   src/lib.rs and fix whatever it surfaces.

 Test and doc updates

 - matching.rs:1228 / :1239: the golden score: 0.98976606 will change. Recompute.
 - matching.rs:1133-1138: mutual_score(default, default) == 1.0 should still hold — a
   default response is all 1s, so want == have under either operator, and the assertion
   is on the raw/display path. Keep it as a regression guard.
 - docx.rs:300-303: replace the data.build().document.len() == 6711 byte-count golden
   with a structural assertion (table count, and that the name/email/prompt strings appear).
   It breaks on any styling change or docx-rs bump and tells you nothing when it does.
 - New tests: the shortfall operator truth table (want=1,have=4 ⇒ 1.0; want=4,have=1 ⇒ 0.0; want=3,have=4 ⇒ 1.0; want=3,have=2 ⇒ 0.67); calibration is a no-op for a person
   with one candidate; entries_served == max_possible_entries on
   test_data/many_generated.csv; --diagnostics-format json round-trips through
   serde_json.
 - DESIGN.md: line 296 documents the cross-match as 1 − |a_F − b_E| for all 8 pairs —
   correct it and name which items are bipolar. Add the calibration step to the Algorithm
   section and the new metrics to the design summary. Line 366's open question ("place a
   random placebo match … useful as a baseline") is partly answered by metric C.
 - README.md:79-145: the sample diagnostics block is already stale (missing the
   Random seed used for generation: first line; zero_appearances now also prints the
   participant list). Regenerate it from a real run once the new metrics land.

 ---

 Verification

 1. cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings, cargo test
    (mirrors .githooks/pre-push and ci.yaml).
 2. wasm-pack build --target web --out-dir /tmp/wasm-pkg — the wasm export signature is
    unchanged (no new flag), but the GUI must still build and run against the new scoring.
 3. Capture diagnostics before and after on the real data and diff:
    ./target/release/matchmaking ~/Downloads/out.csv -d --seed 1 --diagnostics-format json
    Expected direction of travel, against the baseline I measured:

 | Metric                  | Baseline | Expectation        |
 |-------------------------|----------|--------------------|
 | algorithm_limited_short | (new)    | 0                  |
 | headroom_ratio          | 0.994    | stays ≥ 0.99       |
 | demand_gini             | ~0.549   | drops toward ~0.30 |
 | demand_zero             | 19       | drops toward ~2    |
 | zero_appearances        | 9        | well below 9       |
 | rank_regret_mean / p95  | 0.92 / 4 | well below         |
 | cap_relaxed             | true     | likely false       |
 | mutual_rate             | 52.8%    | up                 |

 4. To isolate the two fixes' individual effect on the real data, temporarily stash Phase 2
    (git stash the calibration diff) and diff diagnostics with Phase 1 alone, then restore
    and diff with both — no permanent flag needed, this is a one-time check.
 5. Sanity-check the churn by eye — diff the -o json shortlists before and after and spot
    check two or three people who said "doesn't matter" on the dry-humor or artistic items
    (32 and 14 people respectively). Their lists should change the most.
 6. Re-run the seed sweep (30 seeds) after the changes to confirm the conclusion still holds
    and no restart loop is warranted.
 7. All work happens on the existing ekunz-better-scoring branch (already checked out;
    working tree is clean).
