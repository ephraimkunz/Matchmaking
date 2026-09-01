% ============================================================
% Dating Match Scheduler
% TODO: Handle walkins, attendance and unscheduled people: 
%For walk-ins: the fix is to relax the one constraint that currently forbids any pairing not backed by a list entry:
% Only allow a pair if at least one person listed the other.
%constraint
%    forall(p, q in PEOPLE, r in ROUNDS where p < q) (
%        paired[p, q, r] ->
%            (wants[p, q] \/ wants[q, p])
%    );
% Add a new constraint to try to add men and women.
% Also add to output printing each match each round.
% ============================================================

int: N;
set of int: PEOPLE = 1..N;

int: R;
set of int: ROUNDS = 1..R;

array[1..N] of string: names;

% wants[p, q] = true if person p listed person q.
array[PEOPLE, PEOPLE] of bool: wants;

% ============================================================
% DECISION VARIABLES
% ============================================================

% paired[p, q, r] means p and q are paired in round r.
%
% We only use p < q conceptually. The symmetry constraints below
% make paired[p,q,r] == paired[q,p,r].
array[PEOPLE, PEOPLE, ROUNDS] of var bool: paired;


% ============================================================
% BASIC VALIDITY CONSTRAINTS
% ============================================================

% Nobody can be paired with themselves.
constraint
    forall(p in PEOPLE, r in ROUNDS) (
        not paired[p, p, r]
    );


% Make the matrix symmetric.
constraint
    forall(p, q in PEOPLE, r in ROUNDS) (
        paired[p, q, r] = paired[q, p, r]
    );


% Only allow a pair if at least one person listed the other.
constraint
    forall(p, q in PEOPLE, r in ROUNDS where p < q) (
        paired[p, q, r] ->
            (wants[p, q] \/ wants[q, p])
    );


% ============================================================
% CONSTRAINT: AT MOST ONE PARTNER PER ROUND
% ============================================================

constraint
    forall(p in PEOPLE, r in ROUNDS) (
        sum(q in PEOPLE where q != p)(
            bool2int(paired[p, q, r])
        ) <= 1
    );


% ============================================================
% CONSTRAINT: NO REPEATED PAIRS
% ============================================================

constraint
    forall(p, q in PEOPLE where p < q) (
        sum(r in ROUNDS)(
            bool2int(paired[p, q, r])
        ) <= 1
    );


% ============================================================
% DERIVED VALUES: TOTAL MATCHES PER PERSON
% ============================================================

% Number of rounds in which person p is paired.
array[PEOPLE] of var 0..R: match_count;

constraint
    forall(p in PEOPLE) (
        match_count[p] =
            sum(r in ROUNDS, q in PEOPLE where q != p)(
                bool2int(paired[p, q, r])
            )
    );


% ============================================================
% DERIVED VALUES: UNSCHEDULED PER ROUND
% ============================================================

array[ROUNDS] of var 0..N: unscheduled_count;

constraint
    forall(r in ROUNDS) (
        unscheduled_count[r] =
            sum(p in PEOPLE) (
                bool2int(
                    sum(q in PEOPLE where q != p)(bool2int(paired[p, q, r])) = 0
                )
            )
    );


% ============================================================
% DERIVED VALUES: HONORED MATCHES
% ============================================================

% Number of rounds where p talks to someone p personally listed.
array[PEOPLE] of var 0..R: honored_count;

constraint
    forall(p in PEOPLE) (
        honored_count[p] =
            sum(
                r in ROUNDS,
                q in PEOPLE where q != p /\ wants[p, q]
            )(
                bool2int(paired[p, q, r])
            )
    );


% ============================================================
% DIMINISHING RETURNS: MATCH COVERAGE
% ============================================================

% These mean:
%
% has_match_1[p] = p got at least one match
% has_match_2[p] = p got at least two matches
% has_match_3[p] = p got three matches

array[PEOPLE] of var bool: has_match_1;
array[PEOPLE] of var bool: has_match_2;
array[PEOPLE] of var bool: has_match_3;

constraint
    forall(p in PEOPLE) (
        has_match_1[p] <-> match_count[p] >= 1
    );

constraint
    forall(p in PEOPLE) (
        has_match_2[p] <-> match_count[p] >= 2
    );

constraint
    forall(p in PEOPLE) (
        has_match_3[p] <-> match_count[p] >= 3
    );


% ============================================================
% DIMINISHING RETURNS: HONORED MATCH COVERAGE
% ============================================================

% These mean:
%
% has_honored_1[p] = p got at least one person from their list
% has_honored_2[p] = p got at least two people from their list
% has_honored_3[p] = p got three people from their list

array[PEOPLE] of var bool: has_honored_1;
array[PEOPLE] of var bool: has_honored_2;
array[PEOPLE] of var bool: has_honored_3;

constraint
    forall(p in PEOPLE) (
        has_honored_1[p] <-> honored_count[p] >= 1
    );

constraint
    forall(p in PEOPLE) (
        has_honored_2[p] <-> honored_count[p] >= 2
    );

constraint
    forall(p in PEOPLE) (
        has_honored_3[p] <-> honored_count[p] >= 3
    );


% ============================================================
% MUTUAL MATCHES
% ============================================================

% Number of mutual matches actually scheduled.
var int: mutual_matches =
    sum(
        p, q in PEOPLE,
        r in ROUNDS
        where p < q /\ wants[p, q] /\ wants[q, p]
    )(
        bool2int(paired[p, q, r])
    );


% ============================================================
% OBJECTIVE COMPONENTS
% ============================================================

% ------------------------------------------------------------
% 1. MATCH COVERAGE
%
% Strongly prefer giving everyone at least one match.
%
% Then prefer a second match.
%
% Then prefer a third.
% ------------------------------------------------------------

var int: coverage_score =
      10000 * sum(p in PEOPLE)(bool2int(has_match_1[p]))
    +  1000 * sum(p in PEOPLE)(bool2int(has_match_2[p]))
    +   100 * sum(p in PEOPLE)(bool2int(has_match_3[p]));


% ------------------------------------------------------------
% 2. HONORED MATCH COVERAGE
%
% Strongly prefer everyone getting at least one person
% from THEIR OWN list.
%
% Then spread second and third honored matches.
% ------------------------------------------------------------

var int: honored_score =
       80 * sum(p in PEOPLE)(bool2int(has_honored_1[p]))
    +   8 * sum(p in PEOPLE)(bool2int(has_honored_2[p]))
    +   1 * sum(p in PEOPLE)(bool2int(has_honored_3[p]));


% ============================================================
% FINAL OBJECTIVE
% ============================================================

solve maximize
    coverage_score
    + honored_score
    + 2 * mutual_matches;


% ============================================================
% OUTPUT
% ============================================================

output [
    "Coverage score: ", show(coverage_score), "\n",
    "Honored score: ", show(honored_score), "\n",
    "Mutual matches: ", show(mutual_matches), "\n\n",

    "\nSchedule:\n",

    concat([
        "Round " ++ show(r) ++
        " (unscheduled: " ++ show(unscheduled_count[r]) ++ "):\n" ++
        concat([
            "  \(names[p]) <-> \(names[q])" ++
            (if wants[p,q] /\ wants[q,p]
             then " (mutual)"
             elseif wants[p,q]
             then " (p chose q)"
             else " (q chose p)"
             endif) ++
            "\n"
            | p, q in PEOPLE where p < q /\ fix(paired[p,q,r])
        ])
        | r in ROUNDS
    ]),

    "\nFinal summary:\n",

    concat([
        "\(names[p]): attended " ++ show(match_count[p]) ++ "/" ++ show(R) ++
        " rounds, " ++ show(honored_count[p]) ++ " with someone on their list\n"
        | p in PEOPLE
    ])
];
