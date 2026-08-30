# Singles ward date match design

## Problem statement

I'm hosting an event in my LDS mid-singles ward for 100-200 people. Participants fill out a questionnaire and receive a shortlist of compatible opposite-sex matches. We may then host a speed-dating event of some kind, like a 3-course meal where each person picks someone from their shortlist for each course. We're all the same religion and the goal is to surface compatible long-term dating prospects that could lead to marriage.

## Prior art and why this approach

Matchbox (match.box) is a commercial solution but too expensive for our ward: $95 + $8/guest for the elite tier, which would be needed for multiple rounds of matching.

Some colleges, such as Stanford, run the Marriage Pact (marriagepact.com). The creators went on to commercialize it as Matchbox above. You have to sign up to host an event though, and I don't think our ward is big enough that they'd consider us.

The Marriage Pact and Matchbox question banks are not public, but a partial set has been gathered from Reddit (preserved in the section below for reference).

[Stable matching](https://en.wikipedia.org/wiki/Stable_matching_problem) is a related problem but requires every participant to rank every other participant, which is impractical at this scale. The goal is clustering compatible individuals and producing high-quality shortlists. Optionally, a follow-up Gale-Shapley round could run on top of those shortlists, but that is out of scope for v1.

Items kept in mind during the design:

- **Length:** 60-90 questions seems to be the sweet spot. The current questionnaire has ~75 scored questions plus dealbreakers and free-response.
- **Selective importance weighting:** OKCupid-style importance applied to a subset of high-signal questions, not all of them. Per-person caps prevent gaming by marking everything "must agree."
- **Coarse 4-point scale:** Eliminates mood-swing noise and forces a lean - there is no neutral midpoint.

## Marriage Pact questions (gathered from Reddit, for reference)

These are the questions I started from. Many were adapted, dropped, or reworded for the final questionnaire. Kept here so the next maintainer can see where the question bank came from.

Expensive dates would be more fun

If you do nothing for an entire day, how do you feel?

I would go on a spontaneous trip, even if that meant putting off my responsibilities

It is more important to protect someone's feelings than to tell the truth

I keep some "friends" because they might be useful to me in the future

I want a family with ____ children

Social activism is important to me

I would consider my friends quiet

Do you prefer to be the person who is left on their wedding day or the person who leaves someone on their wedding day?

People in positions of authority are usually right

I prefer politically incorrect humor

There is a place for revenge when someone has wronged me

I always vote

I say what is bothering me, even if it makes my partner uncomfortable

Some things are simply black and white

The phrase "I love you" is a promise

I'd rather surround myself with people who are... (simple or complex)

My partner can be "just friends" with an ex

I can't go to sleep if my partner is upset with me

I go to great lengths to minimize my harm to the planet

Everything will eventually be explained by science

I would want my partner to share their location with me

I would keep a gun in the house

It's important that I make more money than my peers

I would end a friendship over differing political views

I like to play games that involve disclosing intimate thoughts & feelings

If there were a red light, but no one was on the road, I would go

Are you smarter than most people at church?

I do whatever it takes to get ahead

I would be okay if I spent my life doing good for others, but did not receive recognition for it

My friends often come to me for advice on their problems

I would never stalk my partner's social media activity

I want to have an extravagant wedding

My partner should enjoy hanging out with my family, even when I'm not there

My kids should attend private school

No one can be truly self-made

It's okay to speak badly of other people, as long as they don't find out

I always try to sound eloquent even in non-academic conversations

It is ok for my partner to have extreme crushes on celebrities

I am the definition of the life of the party

It's best to split the bill on the first date

When someone vents to me, I first offer (emotions / advice)

Everyone deserves my empathy

I count every penny I spend

I have told someone I love them even if I didn't mean it

AI is a net good for society

It's important my partner has an artistic side

I usually find it harder to (chill out / get hyped up)

I have a dry sense of humor

I'd rather have a partner who (meticulously plans / goes with the flow)

I consider myself to be an adult

It's important my parents approve of my partner

I'd rather ghost somebody than outright reject them

The world needs (more realism / more imagination)

I avoid burning bridges at all cost

I run most major decisions by my parents

Billionaires should not exist

I would rather fail than cheat on an exam

I'm the most important person in my life

I believe gossiping about others is harmless fun

Expressing myself through fashion or style is important to me

My profession is a defining part of who I am

Watching or playing sports is an important part of my identity

I pay close attention to my diet and nutrition

Staying active and exercising is important to me

I enjoy lazy days with no plans or responsibilities

I prefer having a structured daily routine

I like to indulge in non-essential purchases

I enjoy treating myself, even if it's impractical

I am highly goal-oriented in my studies or career

I like making long-term plans for my future

I prioritize work-life balance over professional ambition

Financial stability is more important to me than chasing passion projects

I prefer a small, close-knit group of friends over a large social circle

I enjoy social media and actively engage with it

I love planning and hosting gatherings or parties

I value deep, meaningful conversations over casual small talk

I enjoy playful teasing and harmless pranks with my friends

I often check in on my friends to see how they're doing

I'm comfortable with my friends having different opinions and beliefs

I feel one should choose to buy from local businesses over large corporations

I'm a night owl

I need friends who respond quickly to messages

I'm uncomfortable with excessive physical affection (hugging, etc.)

I love celebrating birthdays and important milestones with friends

I prefer cozy nights in over going out and socializing

I enjoy discussing deep topics like philosophy, science, or psychology

I like participating in group activities like sports, board games, or trivia

I enjoy dark humor and sarcasm

I'd rather blend in than stand out in a group

I'm always excited to try exotic and unusual foods

I enjoy following and discussing pop culture, TV shows, and movies

I love going to concerts, festivals, or other live events with friends

I find thrill-seeking activities (e.g., skydiving, roller coasters) exciting

I could spend hours getting lost in a book or creative project

---

## Questionnaire structure

### Instructions shown to participants

Answer honestly - there are no right or wrong answers, and others won't be able to see your responses (with the exception of the free-response questions). Most questions use a 4-point scale. For some questions, you'll also rate how important it is to you that your partner feels the same way on that question. The entire questionnaire should take 20 - 40 minutes to complete.

Near the end, you'll write a few short answers to free-response questions. Your responses to these questions will be visible to other people if you appear on their compatibility list.

How it works (if you are curious)

We will take all of the survey data and feed it into our matching algorithm, described here. The result will be a compatibility list for each person who completed the survey, containing, in no particular order, those with whom you are most compatible.

This list is not symmetric - you will not automatically appear on someone else's list just because they appear on yours. Your list shows you who you are most compatible with and their list shows who they are most compatible with, and those two sets may have varying degrees of overlap.

### Demographics

Name, gender, age. Email is collected separately by Google Forms. Age is restricted to `Age::MIN_AGE`-`Age::MAX_AGE` (checked at the parser-level and by validations in the Google Form).

### Dealbreakers

Hard filters on fundamental life-path compatibility (e.g., children, marriage timeline, geography, religious commitment). These are not scored. Pairs failing any check are removed before scoring. Filter rules are bidirectional; both people's stated tolerances are checked in the same call.

### Core Values

Questions on values relevant to long-term compatibility. Each carries an importance rating chosen by the respondent, making this the highest-signal section.

### Relationship Dynamics

Questions about how someone operates inside a relationship. A subset are importance-weighted (the high-signal items); the remainder use a fixed weight.

### Lifestyle & Money

These questions cover spending habits, personal ambition, and practical lifestyle preferences. All items carry the same weight. The desired number of children is included as a numeric indicator rather than a strict requirement.

### Self-description

Questions where participants describe their own traits and tendencies. All 15 are scored as direct similarity. Eight of them also feed the Partner Preferences cross-match and score twice.

### Partner Preferences

Questions about what traits and behaviors someone wants in a partner. Eight are cross-matched against the partner's self-description (who you want ≠ who you are). Two are scored as direct similarity.

### Social Style

Questions about social temperament and how someone spends time with others. Direct similarity.

### Interests

Questions about hobbies and recreational interests. Direct similarity, but down-weighted because shared interests are a weaker predictor of long-term compatibility than shared values or relationship style.

### Free-response

Participants choose a few prompts to answer in their own words. Not scored. Used to populate the human-readable match card.

Section order above is documentation-only and does not need to match the live questionnaire's order. The parser matches each section by name, not by position, so this list can be resorted freely without touching `src/parsing.rs`. If you reorder the actual questionnaire, keep `questionnaire_structure.json` (regenerated from the Google Form) and `src/parsing.rs`'s section-parsing call order in sync with each other. This file has no ordering dependency to maintain.

---

## Algorithm

### Inputs

- Each participant's 4-point answers are normalized to 0.0-1.0 by `FourChoiceResponse::normalized()`: `(value - 1) / 3`.
- Children count is normalized by `NumChildren::normalized()`.
- Importance ratings (Core Values and the weighted subset of Relationship Dynamics only) are normalized by `FiveChoiceWeight::normalized()` to the range `MIN_NORMALIZED`-`MAX_NORMALIZED`, so "I don't care if we agree" still counts a little (never zeroes a question) and "We MUST agree on this" caps at `MAX_NORMALIZED` per question.
- An anti-gaming person-level cap (`PERSON_BOOST_CAP`): if a respondent's average importance across all weighted questions exceeds `PERSON_BOOST_CAP`, every weight they set is scaled down proportionally. Pickiness is a still signal; it just can't overwhelm every question.

### Step 1 - Filter

For every opposite-sex pair, evaluate the four dealbreaker rules in order. If any rule fails, the pair is dropped before scoring. The check is bidirectional. Both A's and B's tolerances are evaluated in the same call.

### Step 2 - Directional score

For one direction (A → B), i.e., how well does B satisfy A's preferences:

- For every importance-weighted question (Core Values, first 8 of Relationship Dynamics): compute `similarity = 1 − |a − b|`, multiply by A's capped importance weight and the section weight, accumulate into a running total and weight sum.
- For every fixed-weight question (last 3 of Relationship Dynamics, Lifestyle & Money, Self-description direct, Social Style, Interests): compute `similarity = 1 − |a − b|`, multiply by the section weight, accumulate.
- For each of the 8 cross-match pairs (Partner Preferences → partner's matching Self-description trait): 2 are a genuine preference axis ("plans carefully" vs. "goes with the flow"; "homebody" vs. "very social"), scored as `similarity = 1 − |a_F − b_E|`. The other 6 are an importance scale ("...in a partner matters", 1 = not at all), scored as `similarity = 1 − max(0, a_F − b_E)`: wanting less of a trait than a partner has is never penalized. Both cases are multiplied by `PARTNER_PREFERENCES_SECTION_WEIGHT`.
- For age: compute proximity over the `Age::MIN_AGE`-`Age::MAX_AGE` span, weighted at `AGE_QUESTION_WEIGHT`.
- Return `total / weight_sum`.

Pickiness is signal: A's importance ratings only affect `score(A → B)`.

### Step 3 - Mutual score

```
mutual_score(A, B) = 0.8 × min(score(A→B), score(B→A))
                   + 0.2 × midpoint(score(A→B), score(B→A))
```

The min term punishes one-sided matches where one person would be miserable. The midpoint term breaks ties. With pure averaging, a perfect/poor pair scores the same as two moderate pairs. The second is the better real-world match.

This is the display score shown on match cards (`--print-scores`) and used for the QUALITY diagnostics. It is not, on its own, what decides who ends up on whose shortlist. See Step 4.

### Step 4 - Calibration

Some people score well (or poorly) with almost everyone, independent of the pair fit. A person's own answers can structurally push their score up or down against nearly the whole pool: narrow partner preferences score low against most candidates, answers near the pool's center score higher, regardless of who the specific candidate is. Left uncorrected, this turns candidate ranking into something closer to a popularity contest: on real data collected for this project, as much as 59% of pair-score variance has come from this effect rather than pair-specific compatibility (the `person_effect_share` diagnostic).

Calibration seeks to resolve this substantially but not completely, and costs almost nothing in absolute compatibility to do it. `person_effect_share_calibrated` has come in at roughly half the raw figure or less on real runs: 58.4% → 33.3% on one, 31.3% → 12.6% on the repo's `many_generated.csv` fixture. Meanwhile `headroom_ranking` — the share of any headroom loss attributable to calibration's re-ranking rather than the appearance cap — has read 99.4%–99.7% on those same runs, meaning calibration gave up at most a few tenths of a percent of served score to get there. That combination (large drop in person effect, negligible compatibility cost) is why calibration is something we should do.

Calibration means a second, calibrated rank score is computed alongside the display score, and used only to order each person's candidates:

```
mean_outgoing(A) = mean of score(A → X) over every X A is compatible with
cal(A → B)        = score(A → B) − mean_outgoing(A)
rank_score(A, B)  = 0.8 × min(cal(A→B), cal(B→A)) + 0.2 × midpoint(cal(A→B), cal(B→A))
```

Subtracting each person's own average removes this structural, answer-driven bias from the ranking, without touching the display score participants actually see. It only removes a person's own outgoing average, so it blunts `person_effect_share` rather than eliminating it; `person_effect_share_calibrated` and `headroom_ranking` in the README's `-d` output show exactly how much survives and what it cost, respectively. Someone with fewer than 2 candidates has no meaningful average to subtract, so calibration is a no-op for them.

### Step 5 - Build candidate ranking

For every participant, sort surviving opposite-sex candidates by rank score descending.

### Step 6 - Round-robin shortlist assignment

Each round, shuffle the order of incomplete participants, then give each person their next-best available candidate (by rank score). Skip anyone already on their shortlist, and skip anyone who has been picked `--max-appearances` times. Iterate until every shortlist reaches `--target-shortlist` length, or a full pass makes no progress.

If the algorithm stalls at the current cap, the cap is raised by one and the process repeats to reach `--target-shortlist`. This continues until either every shortlist is full, or the cap has reached `--max-appearances-relaxed` and no further progress is possible. All three values are tunable CLI flags with sensible defaults for the 100-200 person use case.

The per-round shuffle prevents any single person from systematically getting first pick. The appearance cap prevents a few popular profiles from absorbing every shortlist. The `algorithm_limited_short` diagnostic flags a person whose shortlist came up short despite having enough viable candidates — the appearance cap, not the pool, is the reason.

### Step 7 - Render match cards

Each card lists the matched person's name, email, and the free-response hooks they chose. By default shortlists are in randomized order so the participant doesn't anchor on the top score. `--sort-shortlists-by-score` and `--print-scores` expose the underlying ranking when needed.

---

## Design summary table

| Section | Scoring | Section weight constant |
|---|---|---|
| Dealbreakers | Filter only | - |
| Core Values | Importance-weighted similarity | `CORE_VALUES_SECTION_WEIGHT` |
| Relationship dynamics (weighted) | Importance-weighted similarity | `RELATIONSHIP_DYNAMICS_SECTION_WEIGHT` |
| Relationship dynamics (fixed) | Fixed similarity | `RELATIONSHIP_DYNAMICS_SECTION_WEIGHT` |
| Lifestyle & money | Fixed similarity | `LIFESTYLE_MONEY_SECTION_WEIGHT` (children also ×`NUM_CHILDREN_QUESTION_WEIGHT`) |
| Self-description | Direct similarity (all 15; 8 also feed Partner Preferences cross-match) | `SELF_DESCRIPTION_SECTION_WEIGHT` |
| Partner preferences | Cross-match + direct | `PARTNER_PREFERENCES_SECTION_WEIGHT` |
| Social style | Fixed similarity | `SOCIAL_STYLE_SECTION_WEIGHT` |
| Interests | Fixed similarity | `INTERESTS_SECTION_WEIGHT` |
| Free-response | Not scored | - |
| Demographics - Age | Linear proximity over `MIN_AGE`-`MAX_AGE` span | `AGE_QUESTION_WEIGHT` |

**Importance-weighted questions:** 22 (Core Values + first 8 of Relationship Dynamics). Per-question cap `MAX_NORMALIZED`; per-person average cap `PERSON_BOOST_CAP`.

**Cross-match items:** 8 total. 2 bipolar (`CROSSMATCH_IS_BIPOLAR`), scored `1 − |want − have|`. 6 importance-scale, scored `1 − max(0, want − have)`.

## Quality diagnostics

Beyond the POOL/CONVERGENCE/QUALITY metrics documented in the README's example output, four things are worth tracking as design-level concerns, not just run-level output:

- **Headroom** (`headroom_ratio` and friends): served score as a fraction of what each person's own best-possible candidates would have given them, ignoring the appearance cap entirely. This is the number that answers "would a smarter assignment algorithm help?" — on real data collected for this project it has run at ~99%, meaning the round-robin assignment already captures nearly all the quality the pool allows. `headroom_ratio` decomposes exactly into `headroom_ranking * headroom_assignment`: the first isolates calibration's cost in absolute compatibility, the second isolates the appearance cap/round-robin's cost given calibration's ranking as the ideal.
- **Feasibility** (`max_possible_entries`, `pool_limited_short`, `algorithm_limited_short`): separates a short shortlist caused by a thin candidate pool (not fixable by tuning) from one caused by the appearance cap ceiling leaving a saturated candidate unreachable.
- **Separability** (`pair_score_stddev`, `top_gap_mean`, `top_gap_in_sds`): how much a person's best candidate actually stands out from their `target_shortlist`-th best, in units of the overall score spread. When this is small, the ranking near the top of a shortlist is mostly noise, and the default randomized shortlist order (rather than `--sort-shortlists-by-score`) is the more honest presentation.
- **Person effects and demand concentration** (`person_effect_share`, `person_effect_share_calibrated`, `demand_max`, `demand_zero`, `demand_gini`): `person_effect_share` is a property of the raw scoring math — how much of the spread comes from a person's own answers and weights scoring them high or low against nearly everyone, rather than genuine pair fit. `person_effect_share_calibrated` is the identical fit on the rank score, i.e. what survives calibration (Step 4). `demand_gini` and `demand_zero` show the resulting concentration under the calibrated ranking: how unevenly a few people end up wanted by everyone else's top candidates.

## Key design decisions

| Decision | Rationale |
|---|---|
| 4-point input, no midpoint | Forces a lean; eliminates mood-swing noise. |
| Linear similarity on bucketed inputs | Simple and stable on a coarse scale. |
| Importance weight `MIN_NORMALIZED`-`MAX_NORMALIZED` | "I don't care" never zeroes a question; "must match" caps at `MAX_NORMALIZED` per question. |
| Per-person boost cap (`PERSON_BOOST_CAP`) | Prevents gaming by marking everything "must match". |
| `mutual_score = 0.8·min + 0.2·midpoint` | Punishes one-sided matches. With pure averaging, a perfect/poor pair scores the same as two moderate pairs. |
| Partner Preferences cross-matched against partner's Self-description | Who you want ≠ who you are. |
| Self-description at `SELF_DESCRIPTION_SECTION_WEIGHT` (0.6) | The 8 cross-matched Self-description items score twice (direct + Partner Preferences cross-match); the lower weight compensates. |
| Children count at `NUM_CHILDREN_QUESTION_WEIGHT` × `LIFESTYLE_MONEY_SECTION_WEIGHT` | A real signal, but a conversation - not an automatic disqualification. |
| Round-robin with per-round shuffle | Eliminates first-pick order bias. |
| `--max-appearances` cap, ramped +1 per stall to `--max-appearances-relaxed` | Prevents popular profiles from dominating every shortlist; quality degrades as little as necessary before expanding capacity. |
| No Gale-Shapley | Stable matching requires full mutual rankings; impractical at this scale. Shortlists are sufficient for the speed-dating event. |
| Cross-match split: bipolar vs. shortfall-only | 6 of the 8 cross-matched Partner Preferences items are importance scales ("...matters"), not preference axes. Scoring them as bipolar treated "doesn't matter to me" as an aversion to be penalized whenever a partner had the trait. |
| Per-person mean-centering (rank score, kept separate from the display score) | Corrects for each person's own answers and weights structurally scoring them high or low against nearly everyone. Ensures someone with narrow preferences or off-center answers doesn't get an artificial penalty (or edge) purely from that, relative to someone closer to the pool's center. Measured cost in absolute compatibility (`headroom_ranking`) has been negligible (99.4%–99.7%) on real runs — subtracting a constant can't reorder a person's own candidate list, it only changes which side binds the `min()` in `combine`, so nobody's actual stated preferences get overridden. |
| No restart/hill-climbing over the round-robin's randomness | Measured `headroom_ratio` on real data sits around 99%: the assignment already captures nearly all the score the pool allows, so re-running with different seeds and keeping the best has almost nothing to gain. |

## Open questions / TODO

- Clearer breadwinning / gender-roles question? Currently approximated through earnings and ambition questions.
- Place a random placebo match on each shortlist? Useful as a baseline for evaluating whether the algorithm beat random. `top_gap_in_sds` (how much the top of a shortlist stands out from the rest) is a related but narrower signal — it doesn't establish whether the *scoring itself* beats chance, only whether ordering within a shortlist matters once the scoring is fixed.
- For the speed-dating event itself: alternate who hosts and who roams at each course, so each side feels like they have agency.
