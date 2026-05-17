# Singles ward date match research

Problem statement: I’m hosting an event in my LDS mid-singles ward (ages 26-35) for 100 - 200 people. They will fill out a questionnaire to get a shortlist of people they may be compatible with (opposite sex). We may then host a speed dating event of some kind, like a 3 course meal where they choose someone from their list from each of the 3 courses. We are all the same religion and the idea is to provide compatible long-term dating prospects that will lead to marriage. Below is my proposal. 

Matchbox (match.box) is a commercial solution but would be too expensive for our ward: $95 + $8 / guest for elite, which would be needed for multiple grounds of matching.

Some colleges, such as Stanford, run the Marriage Pact (marriagepact.com). The creators went on to commercialize as Matchbox above. You have to sign up to host an event though, and I don’t think our ward is big enough they would consider.

The questions to marriage pact are not public, nor are the matchbox questions, but some subset has been posted on reddit. I’ve compiled them below.
 
https://en.wikipedia.org/wiki/Stable_matching_problem is a related problem, but requires all participants to preference rank each other. This O(n^2) algorithm creates pairings that ensure that there doesn’t exist any pair which both prefer each other to their current partner.

It’s impractical to have everyone in the ward rank everyone else. So, it seems like something like the following could be used:
1. Have participants fill out a questionnaire.
2. Use the questionnaire to generate a list of top n candidates, ranked by compatibility.
3. Use this top n list in various speed-dating activities.
4. Optionally, each user can rank their short-lists and we can use Gale-Shapely to solve the stable matching problem on top of that.

From this we see that the problem isn’t algorithmic (stable matching problem), but is about clustering together compatible individuals and generating high-quality shortlists.

Items to keep in mind:
Length of questionnaire - 60 - 90 questions seems best
Allow users to weight certain questions, but not all to avoid fatigue. Similar to OKCupid importance weighting that user picks (1-5) (“importance weighting works best when applied selectively to high-signal questions” ““How important is it that your partner agrees with you?”” (irrelevant, a little important, somewhat important, very important, mandatory)

## Marriage pact questions (gathered from Reddit):

Expensive dates would be more fun
If you do nothing for an entire day, how do you feel?
I would go on a spontaneous trip, even if that meant putting off my responsibilities
It is more important to protect someone's feelings than to tell the truth
I keep some “friends” because they might be useful to me in the future
I want a family with ____ children
Social activism is important to me
I would consider my friends quiet
Would you rather be left at the altar or leave someone at the altar?
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
I would never stalk my partner’s social media activity
I want to have an extravagant wedding
My partner should enjoy hanging out with my family, even when i’m not there
My kids should attend private school
No one can be truly self-made
It’s okay to speak badly of other people.. as long as they don’t find out
I always try to sound eloquent even in non-academic conversations
It is ok for my partner to have extreme crushes on celebrities
i am the definition of the life of the party 
its best to split the bill on the first date 
when someone vents to me, i first offer (emotions 1, advice 7) 
everyone deserves my empathy 
i count every penny i spend 
i have told someone i love them even if i didn't mean it 
AI is a net good for society 
its important my partner has an artistic side 
i usually find it harder to (chill out 1 or get hyped up 7) 
i have a dry sense of humor 
id rather have a partner who (meticulously plans 1, goes with the flow 7) 
i consider myself to be an adult 
its important my parents approve of my partner 
id rather ghost somebody than outright reject them 
the world needs (more realism 1, more imagination 7) 
i avoid burning bridges at all cost 
i run most major decisions by my parents 
billionares should not exist 
i would rather fail than cheat on an exam 
im the most important person in my life 
I believe gossiping about others is harmless fun.
Expressing myself through fashion or style is important to me.
My profession is a defining part of who I am.
Watching or playing sports is an important part of my identity.
I pay close attention to my diet and nutrition.
Staying active and exercising is important to me.
I enjoy lazy days with no plans or responsibilities.
I prefer having a structured daily routine.
I like to indulge in non-essential purchases.
I enjoy treating myself, even if it’s impractical.
I am highly goal-oriented in my studies or career.
I like making long-term plans for my future.
I prioritize work-life balance over professional ambition.
Financial stability is more important to me than chasing passion projects.
I prefer a small, close-knit group of friends over a large social circle.
I enjoy social media and actively engage with it.
I love planning and hosting gatherings or parties.
I value deep, meaningful conversations over casual small talk.
I enjoy playful teasing and harmless pranks with my friends.
I often check in on my friends to see how they’re doing.
I’m comfortable with my friends having different opinions and beliefs.
I feel one should choose to buy from local businesses over large corporations.
I’m a night owl.
I need friends who respond quickly to messages.
I’m uncomfortable with excessive physical affection (hugging, etc.).
I love celebrating birthdays and important milestones with friends.
I prefer cozy nights in over going out and socializing.
I enjoy discussing deep topics like philosophy, science, or psychology.
I like participating in group activities like sports, board games, or trivia.
I enjoy dark humor and sarcasm.
I’d rather blend in than stand out in a group.
I’m always excited to try exotic and unusual foods.
I enjoy following and discussing pop culture, TV shows, and movies.
I love going to concerts, festivals, or other live events with friends.
I find thrill-seeking activities (e.g., skydiving, roller coasters) exciting.
I could spend hours getting lost in a book or creative project.


## Matchmaking System — Final Design v4

📌 INSTRUCTIONS (shown to participants)
Answer honestly — there are no right or wrong answers. Most questions use a 4-point scale. For some questions you'll also rate how much this matters to you in a partner — take your time on those, they have the most impact on your matches. At the end you'll write a few short answers that will appear on your match card.
Scale guide shown on form:
Option	Meaning
1	Strongly disagree / Never / Not at all
3	Lean disagree / Rarely
5	Lean agree / Often
7	Strongly agree / Always / Very much
Note: 4-point input (1/3/5/7 only) shown to participants — midpoint removed intentionally to force a lean.

Section 0 - demographics
Name
Gender
Age

SECTION A — DEALBREAKERS
Hard filters only. Not scored. Incompatible pairs removed before any scoring.
#	Question	Format
A1	I want to have children	Yes / No / Open to it
A2	I'd like to be married within	0–2 yrs / 2–5 yrs / 5+ yrs
A3	I intend to stay in Cache valley long-term.	Yes / No / Depends
A4	My religious commitment level	1 (Cultural) → 5 (Deeply devout)
A4b	I need my partner's religious commitment to be	Same level / Within 1 level / Flexible
Filter logic:
* A1: Remove if one says No and other says Yes. Open to it passes with either.
* A2: Remove if windows don't overlap (0–2 yrs vs 5+ yrs fails; all adjacent pairs pass).
* A3: Remove if one says No and the other says Yes. 
* A4/A4b: Remove only if B falls outside A's tolerance or A falls outside B's tolerance.

 SECTION B — CORE VALUES
14 questions. Each has a 4-point agreement scale plus importance rating. Importance: 0 = Doesn't matter · 1 = A little · 2 = Somewhat · 3 = Very · 4 = Must match
Importance scale: How important is it that your partner feels the same way as you do on this?
0: I don't care if we agree.
1: A little.
2: Somewhat.
3: Very.
4: We must agree on this.

#	Question	1	7
B1	Protecting feelings matters more than blunt honesty	Honesty always	Feelings first
B2	Social activism is important to me	Not at all	Central to my life
B3	There is a place for revenge when someone wrongs you	Never	Absolutely
B4	Some things are simply black and white	Everything is nuanced	Some things are absolute
B5	The phrase "I love you" is a promise	Just an expression	A serious commitment
B6	I go to great lengths to minimize harm to the planet	Not a priority	Shapes daily choices
B7	I would keep a gun in the house	Never	Yes
B8	I would end a friendship over political differences	Never	Yes
B9	No one can be truly self-made	Disagree	Agree
B10	Everyone deserves my empathy	Disagree	Agree
B11	I would rather fail than cheat	Disagree	Agree
B12	I am the most important person in my own life	Disagree	Agree
B13	I prefer politically incorrect humor	Strongly dislike	Love it
B14	Buying local over corporate matters to me	Never think about it	Always prioritize
SECTION C — RELATIONSHIP DYNAMICS
Importance-weighted (8):
#	Question	1	7
C1	I say what's bothering me even if it makes my partner uncomfortable	I hold back	I always speak up
C2	I can't sleep if my partner is upset with me	Doesn't affect me	I need resolution
C3	My partner can be just friends with an ex	Never okay	Completely fine
C4	I'd want my partner to share their location with me	Not at all	Yes, always
C5	My partner should enjoy spending time with my family without me	Not important	Very important
C6	My parents' approval of my partner matters to me	Not at all	Essential
C7	I run major decisions by my parents	Never	Always
C8	I'd rather ghost than directly reject someone	Always direct	Ghost every time
Fixed weight (3):
#	Question	1	7
C9	I avoid burning bridges at all cost	No, I'll cut people off	Yes, always
C10	I check in on friends regularly	Rarely	Very often
C11	I need friends who respond quickly to messages	Doesn't matter	Very important

 SECTION D — LIFESTYLE & MONEY
Fixed weight. No importance slider.
#	Question	1	7
D1	Expensive dates are more fun	Disagree	Agree
D2	I count every penny I spend	Never	Always
D3	I like to indulge in non-essential purchases	Never	Regularly
D4	Financial stability matters more than chasing passion	Disagree	Agree
D5	It matters that I earn more than my peers	Not at all	Very much
D6	I want an extravagant wedding	Simple ceremony	Big lavish event
D7	My kids should attend private school	No preference	Strongly yes
D8	I enjoy treating myself even when impractical	Never	Often
D9	I'd like to have ___ children	0 / 1 / 2 / 3 / 4 / 5 / 6 / 7 / 8 / 9 (scored 0–9, weight 0.75×)	
 SECTION E — SELF-DESCRIPTION
Fixed weight at 0.6× (reduced from 0.8 — E scores twice: directly and via F cross-match).
#	Question	1	7
E1	I tend to plan things carefully	Go with the flow	Meticulous planner
E2	I have an artistic or creative side	Not really	Very much so
E3	I am energetic and outgoing	Quiet homebody	Life of the party
E4	I am highly goal-oriented and driven	I take life as it comes	Very ambitious
E5	I prefer structured daily routines	Never	Always
E6	Idle days with no plans feel	Restorative (1)	Uncomfortable (7)
E7	I usually find it harder to	Chill out (1)	Get hyped up (7)
E8	I have a dry sense of humor	Not at all	Definitely
E9	I enjoy intellectual debate and sparring	Not my thing	Love it
E10	My profession is a defining part of who I am	Not really	Very much
E11	Diet and nutrition are important to me	I don't think about it	Central to my life
E12	Staying active and exercising matters	Not important	Very important
E13	Fashion and personal style matter to me	Not at all	A lot
E14	Sports are an important part of my identity	Not at all	Absolutely
E15	I'm comfortable being spontaneous over responsible	Never	Always
SECTION F — PARTNER PREFERENCES
Cross-matched against the other person's Section E. Weight 1.0×. F9 and F10 use standard similarity scoring.
#	Question	1	7	Cross-matches
F1	I'd prefer a partner who plans vs. goes with the flow	Spontaneous	Planner	→ E1
F2	An artistic or creative side in a partner matters	Not at all	Very much	→ E2
F3	I'd prefer a partner who is (homebody ↔ social)	Homebody	Very social	→ E3
F4	Ambition in a partner matters to me	Not important	Very important	→ E4
F5	A dry sense of humor in a partner matters	Don't care	Essential	→ E8
F6	An intellectually curious partner matters to me	Not important	Essential	→ E9
F7	A health-conscious partner matters to me	Don't care	Very important	→ E11
F8	An active or fit partner matters to me	Don't care	Very important	→ E12
F9	Splitting the bill on a first date feels right	No, one person pays	Yes, always split	(similarity)
F10	I'm fine with my partner having celebrity crushes	Never okay	Totally fine	(similarity)
 SECTION G — SOCIAL STYLE
Fixed weight 0.8×.
#	Question	1	7
G1	I prefer a small close-knit friend group	Large circle	Small and tight
G2	I enjoy social media and actively engage with it	Never use it	Very active
G3	I love planning and hosting gatherings	Never	All the time
G4	I value deep conversations over casual small talk	Small talk	Deep talks
G5	I enjoy playful teasing with friends	Not my thing	Love it
G6	I'm comfortable with friends who hold different beliefs	Uncomfortable	Completely fine
G7	I enjoy group activities (sports, games, trivia)	Prefer solo	Love group stuff
G8	I prefer cozy nights in over going out	Always out	Always in
 SECTION H — INTERESTS
Fixed weight 0.5×.
#	Question	1	7
H1	I enjoy philosophy, science, or psychology discussions	Not at all	Love it
H2	I follow pop culture, TV, and movies	Not at all	Obsessively
H3	I enjoy concerts, festivals, and live events	Never go	It's my thing
H4	I enjoy thrill-seeking (skydiving, roller coasters)	Avoid entirely	Love it
H5	I could spend hours lost in a book or creative project	Never	All the time
H6	I enjoy dark humor and sarcasm	Dislike it	Love it
H7	I love trying exotic or unusual foods	Stick to what I know	Always adventurous
H8	I enjoy discussing current events	Not my thing	Love it
 SECTION I — DISTINCTIVE HOOKS
Not scored. Participants select exactly 3 to display on their match card. Must include at least 1 from the bolded prompts.
#	Prompt
I2	Unpopular opinion I stand by: ___
I4	What's something you've changed your mind about recently?
I5	I could give a 10-minute talk on ___
I6	Ideal low-effort hangout: ___
I7	My weekend usually looks like: ___
I8	Niche interest most people don't know I have: ___
I9	Something I'm better at than I let on: ___
I10	The thing I find most attractive in a person: ___

 ALGORITHM — FULL PSEUDOCODE
python
import random
from collections import defaultdict

# ─────────────────────────────────────────────
# CONSTANTS
# ─────────────────────────────────────────────

SECTION_WEIGHT = {
‘0’: 0.7, # Demographics - age is reduced weight
    'B': 1.0,   # Core values — full weight
    'C': 1.0,   # Relationship dynamics — full weight
    'D': 0.8,   # Lifestyle & money
    'E': 0.6,   # Self-description (reduced — scores twice via F cross-match)
    'F': 1.0,   # Partner preferences (cross-matched)
    'G': 0.8,   # Social style
    'H': 0.5,   # Interests — half weight
}

D9_WEIGHT = 0.75  # Children count — soft scored, not a dealbreaker

CROSSMATCH_PAIRS = [
    ('F1', 'E1'), ('F2', 'E2'), ('F3', 'E3'),
    ('F4', 'E4'), ('F5', 'E8'), ('F6', 'E9'),
    ('F7', 'E11'), ('F8', 'E12'),
]

SIMILARITY_F    = ['F9', 'F10']
VALID_INPUTS    = {1, 3, 5, 7}       # 4-point bucketed scale
MAX_IMPORTANCE  = 4                   # 0–4 raw importance rating
BOOST_CAP       = 2.0                 # max any question can be boosted
PERSON_BOOST_CAP = 1.5               # max average boost across all weighted
                                      # questions for one person (anti-gaming)

TARGET_SHORTLIST    = 7
MIN_SHORTLIST       = 5
MAX_APPEARANCES     = 12
MAX_APPEARANCES_RELAXED = 14


# ─────────────────────────────────────────────
# STEP 0: INPUT VALIDATION & BUCKETING
# ─────────────────────────────────────────────

def validate_and_bucket(raw_answers):
    """
    Participants answer on 1/3/5/7 only (forced at UI level).
    This function validates and normalizes to 0.0–1.0 for scoring.
    Mapping: 1→0.0, 3→0.33, 5→0.67, 7→1.0
    """
    bucketed = {}
    for q, val in raw_answers.items():
        if q == 'D9':
            # D9 is 0–9 (child count), normalize separately
            bucketed[q] = val / 9.0
            continue
        if val not in VALID_INPUTS:
            raise ValueError(f"Invalid answer {val} for {q} — must be 1, 3, 5, or 7")
        bucketed[q] = (val - 1) / 6.0   # 1→0.0, 3→0.333, 5→0.667, 7→1.0
    return bucketed


# ─────────────────────────────────────────────
# STEP 1: DEALBREAKER FILTER
# ─────────────────────────────────────────────

def passes_dealbreakers(A, B):

    # A1: children intent
    if A.wants_kids == 'No' and B.wants_kids == 'Yes': return False
    if A.wants_kids == 'Yes' and B.wants_kids == 'No': return False

    # A2: marriage timeline — remove if no overlap
    if not timelines_overlap(A.timeline, B.timeline): return False

    # A3: relocation
    if A.relocate == 'No' and B.relocate == ‘Yes’: return False
    if A.relocate == ‘Yes’ and B.relocate == ‘No’: return False

    # A4/A4b: religious commitment — one must fail tolerance
    a_rejects_b = not within_tolerance(B.religion_level, A.religion_level, A.religion_tolerance)
    b_rejects_a = not within_tolerance(A.religion_level, B.religion_level, B.religion_tolerance)
    if a_rejects_b or b_rejects_a: return False

    return True


def timelines_overlap(t1, t2):
    windows = {'0-2': (0, 2), '2-5': (2, 5), '5+': (5, 99)}
    a0, a1 = windows[t1]
    b0, b1 = windows[t2]
    return a0 <= b1 and b0 <= a1


def within_tolerance(their_level, my_level, my_tolerance):
    if my_tolerance == 'flexible': return True
    if my_tolerance == 'same':     return their_level == my_level
    if my_tolerance == '1':        return abs(their_level - my_level) <= 1


# ─────────────────────────────────────────────
# STEP 2: IMPORTANCE WEIGHT WITH CAPS
# ─────────────────────────────────────────────

def get_importance_weight(person, question):
    """
    Convert raw 0–4 importance rating to a weight multiplier.
    - Raw 0 (irrelevant) → 0.25 (still counts a little — avoids zeroing)
    - Raw 4 (must match) → 2.0 (max per-question boost)
    - Clamped to BOOST_CAP regardless.

    Then: person-level cap prevents gaming by marking everything "must match."
    If person's average boost across all weighted questions exceeds
    PERSON_BOOST_CAP, rescale all their weights proportionally down.
    """
    raw = person.importance.get(question, None)
    if raw is None:
        return 1.0   # fixed-weight question

    # Normalize 0–4 → 0.25–2.0
    boost = 0.25 + (raw / MAX_IMPORTANCE) * 1.75
    return min(boost, BOOST_CAP)


def apply_person_boost_cap(person, weighted_questions):
    """
    If average boost across all weighted Qs exceeds PERSON_BOOST_CAP,
    scale all weights down proportionally.
    Returns a dict of question → final weight for this person.
    """
    raw_weights = {q: get_importance_weight(person, q) for q in weighted_questions}
    avg_boost = sum(raw_weights.values()) / len(raw_weights)

    if avg_boost > PERSON_BOOST_CAP:
        scale = PERSON_BOOST_CAP / avg_boost
        return {q: w * scale for q, w in raw_weights.items()}

    return raw_weights


# ─────────────────────────────────────────────
# STEP 3: DIRECTIONAL SCORE  score(A → B)
# ─────────────────────────────────────────────

WEIGHTED_QUESTIONS = (
    [f'B{i}' for i in range(1, 15)] +   # B1–B14
    [f'C{i}' for i in range(1, 9)]       # C1–C8
)

FIXED_SIMILARITY_QUESTIONS = (
    [f'C{i}' for i in range(9, 12)] +   # C9–C11
    [f'D{i}' for i in range(1, 10)] +   # D1–D9
    [f'E{i}' for i in range(1, 16)] +   # E1–E15
    [f'G{i}' for i in range(1, 9)]  +   # G1–G8
    [f'H{i}' for i in range(1, 9)]      # H1–H8
)


def score(A, B):
    total = 0.0
    weight_sum = 0.0

    # Precompute A's capped importance weights for all weighted questions
    a_weights = apply_person_boost_cap(A, WEIGHTED_QUESTIONS)

    # --- Importance-weighted similarity: B and C(weighted) ---
    for q in WEIGHTED_QUESTIONS:
        a_val = A.answers.get(q)
        b_val = B.answers.get(q)
        if a_val is None or b_val is None:
            continue

        diff = abs(a_val - b_val)           # both already normalized 0.0–1.0
        similarity = 1.0 - diff             # range 0.0–1.0 on bucketed scale
        w = a_weights[q] * SECTION_WEIGHT[q[0]]
        total += similarity * w
        weight_sum += w

    # --- Fixed similarity: C(fixed), D, E, G, H ---
    for q in FIXED_SIMILARITY_QUESTIONS:
        a_val = A.answers.get(q)
        b_val = B.answers.get(q)
        if a_val is None or b_val is None:
            continue

        section = q[0]
        diff = abs(a_val - b_val)

        # D9 (children count): different normalization base
        if q == 'D9':
            similarity = 1.0 - diff       # already normalized 0–1 over 0–9 range
            w = D9_WEIGHT * SECTION_WEIGHT['D']
        else:
            similarity = 1.0 - diff
            w = SECTION_WEIGHT[section]

        total += similarity * w
        weight_sum += w

    # --- Cross-match: F preferences vs B's E traits ---
    for (pref_q, trait_q) in CROSSMATCH_PAIRS:
        a_pref = A.answers.get(pref_q)
        b_trait = B.answers.get(trait_q)
        if a_pref is None or b_trait is None:
            continue

        match = 1.0 - abs(a_pref - b_trait)
        w = SECTION_WEIGHT['F']
        total += match * w
        weight_sum += w

    # --- Similarity-only F questions ---
    for q in SIMILARITY_F:
        a_val = A.answers.get(q)
        b_val = B.answers.get(q)
        if a_val is None or b_val is None:
            continue

        similarity = 1.0 - abs(a_val - b_val)
        w = SECTION_WEIGHT['F']
        total += similarity * w
        weight_sum += w

# --- Age Proximity: Section J --- 
# Max spread is 11 years (26-37). We divide by 12 to keep similarity > 0. 
age_diff = abs(A.age - B.age) 
age_similarity = 1.0 - (age_diff / 12.0) 
w_age = SECTION_WEIGHT[‘0’]
total += age_similarity * w_age 
weight_sum += w_age
 

return total / weight_sum if weight_sum > 0 else 0.0


# ─────────────────────────────────────────────
# STEP 4: MUTUAL SCORE
# ─────────────────────────────────────────────

def mutual_score(A, B):
    """
    Averaged (not summed) so score is not inflated by question count.
    Asymmetry preserved: A's importance weights dominate score(A→B),
    B's weights dominate score(B→A). Average balances them.
    """
    return 0.5 * score(A, B) + 0.5 * score(B, A)


# ─────────────────────────────────────────────
# STEP 5: BUILD ALL CANDIDATE PAIRS
# ─────────────────────────────────────────────

def build_scored_pairs(participants):
    """
    Opposite-sex pairs only (adjust if needed).
    Both directions of dealbreaker checked.
    """
    pairs = {}

    males   = [p for p in participants if p.gender == 'M']
    females = [p for p in participants if p.gender == 'F']

    for A in males:
        for B in females:
            if not passes_dealbreakers(A, B): continue
            if not passes_dealbreakers(B, A): continue
            pairs[(A.id, B.id)] = mutual_score(A, B)

    return pairs


# ─────────────────────────────────────────────
# STEP 6: ROUND-ROBIN SHORTLIST ASSIGNMENT
# ─────────────────────────────────────────────

def assign_shortlists(participants, pairs, cap=MAX_APPEARANCES):
    """
    Round-robin across all participants so no one person
    consistently gets priority across all 7 picks.
    Each round: shuffle order, give each person their next-best
    available match (respecting appearance cap).
    """
    appearance_count = defaultdict(int)
    shortlists = defaultdict(list)

    # Precompute each person's ranked candidate list (descending score)
    ranked_candidates = {}
    for person in participants:
        pid = person.id
        candidates = []
        for (a, b), s in pairs.items():
            if a == pid:   candidates.append((b, s))
            elif b == pid: candidates.append((a, s))
        candidates.sort(key=lambda x: x[1], reverse=True)
        ranked_candidates[pid] = candidates

    def next_available(pid, current_cap):
        for (other_id, score_val) in ranked_candidates[pid]:
            if other_id in shortlists[pid]: continue
            if appearance_count[other_id] >= current_cap: continue
            return other_id
        return None

    incomplete = set(p.id for p in participants)

    while incomplete:
        order = list(incomplete)
        random.shuffle(order)           # re-shuffle every round — no persistent priority

        made_progress = False
        for pid in order:
            if len(shortlists[pid]) >= TARGET_SHORTLIST:
                incomplete.discard(pid)
                continue

            match = next_available(pid, cap)
            if match:
                shortlists[pid].append(match)
                appearance_count[match] += 1
                made_progress = True

        # If no progress was made this round, relax cap and retry
        if not made_progress:
            cap = MAX_APPEARANCES_RELAXED
            order = list(incomplete)
            random.shuffle(order)
            for pid in order:
                if len(shortlists[pid]) >= MIN_SHORTLIST:
                    incomplete.discard(pid)
                    continue
                match = next_available(pid, cap)
                if match:
                    shortlists[pid].append(match)
                    appearance_count[match] += 1
            # Break after relaxed retry regardless — avoid infinite loop
            break

    return shortlists


# ─────────────────────────────────────────────
# STEP 7: BUILD MATCH CARDS
# ─────────────────────────────────────────────

def build_match_cards(person, shortlist, pairs, participant_map):
    """
    score_rank retained in data (1 = best match) for UI layer to use.
    Hooks are the 3 the match selected, including ≥1 distinctive prompt.
    """
    cards = []
    for rank, match_id in enumerate(shortlist, start=1):
        match = participant_map[match_id]
        pid = person.id
        pair_key = (pid, match_id) if (pid, match_id) in pairs else (match_id, pid)
        cards.append({
            'match_id':   match_id,
            'name':       match.name,
            'hooks':      match.selected_hooks,   # 3 chosen by match
            'score_rank': rank,                   # 1 = highest score; filtered in UI
            'raw_score':  pairs.get(pair_key, 0),
        })
    return cards


# ─────────────────────────────────────────────
# FULL PIPELINE
# ─────────────────────────────────────────────

def run_matchmaking(raw_participants):
    # 1. Bucket and validate all answers
    for p in raw_participants:
        p.answers = validate_and_bucket(p.raw_answers)

    # 2. Score all valid pairs
    pairs = build_scored_pairs(raw_participants)

    # 3. Assign shortlists via round-robin
    shortlists = assign_shortlists(raw_participants, pairs)

    # 4. Build match cards
    participant_map = {p.id: p for p in raw_participants}
    results = {}
    for person in raw_participants:
        results[person.id] = build_match_cards(
            person,
            shortlists[person.id],
            pairs,
            participant_map
        )

    return results

 Final Design Summary
Section	Questions	Scoring	Weight
A — Dealbreakers	5	Filter only	—
B — Core Values	14	Importance-weighted similarity	1.0×
C — Relationship (weighted)	8	Importance-weighted similarity	1.0×
C — Relationship (fixed)	3	Fixed similarity	1.0×
D — Lifestyle & Money	9	Fixed similarity (D9 at 0.75×)	0.8×
E — Self-description	15	Fixed similarity + feeds F	0.6×
F — Partner Preferences	10	Cross-match (8) + similarity (2)	1.0×
G — Social Style	8	Fixed similarity	0.8×
H — Interests	8	Fixed similarity	0.5×
I — Hooks	10	Not scored	—
Total scored	~75		
Importance-weighted	22		
🔑 All Design Decisions — Final State
Decision	Rationale
4-point input scale (1/3/5/7)	Eliminates mood-swing noise at source; forces a lean; no midpoint
Linear similarity on bucketed inputs	Clean and non-brittle now that inputs are coarse
Importance weight 0.25–2.0, capped at 2.0× per question	Irrelevant never zeroes; must-match never dominates alone
Person-level boost cap (avg ≤ 1.5×)	Prevents "everything is must-match" from gaming the algorithm
final = 0.5*(AB + BA)	Prevents score inflation from answer count; preserves asymmetry
Asymmetric importance (A's weights in score_AB)	Pickiness is signal; managed by overuse cap not weight blending
Round-robin shortlist with per-round shuffle	Eliminates order bias — no one consistently gets first pick
Overuse cap 12, relaxed to 14 on retry	Prevents popular profiles dominating; fallback avoids short lists
E at 0.6×	Corrects double-counting (E scores directly and via F cross-match)
F cross-match (pref vs. trait, not similarity)	Core insight: who you want ≠ who you are
F at 1.0× (not 1.2×)	F already structurally privileged by cross-matching
H at 0.5×	Shared interests ≠ compatibility
D9 (kids count) soft-scored at 0.75×	Real signal but a conversation, not a dealbreaker
score_rank in data, filtered in UI	Preserves optionality for organiser; prevents anchoring in participant view
1 distinctive hook required from {I1, I2, I3}	Prevents all cards reading identically; labelled "distinctive" not "polarizing"
No Gale-Shapley	Impractical without full mutual ranking; shortlist is sufficient
Age ranking	Prioritize closer in age, but not overwhelmingly

TODO:
Clear enough question about breadwinning / gender roles?
Temptation - put a random person on each person’s short list. Placebo effect. Also, helps us evaluate if we did better than random.

Notes for the event:Alternate who hosts and who roams at each course, so each side feels like they have agency.

Next steps
Have friend group fill out. Make sure algorithm works.
Rollout to our ward
Rollout to 4 ward
