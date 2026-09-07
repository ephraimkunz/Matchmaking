% ============================================================
% Dating Match Scheduler
% ============================================================

int: N;
set of int: PEOPLE = 1..N;

int: R;
set of int: ROUNDS = 1..R;

array[PEOPLE] of bool: attending;
array[PEOPLE] of bool: is_male;
array[PEOPLE] of string: names;

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

% People must be attending to be paired.
constraint
    forall(p, q in PEOPLE, r in ROUNDS where p < q) (
        paired[p, q, r] -> (attending[p] /\ attending[q])
    );


% Make the matrix symmetric.
constraint
    forall(p, q in PEOPLE, r in ROUNDS) (
        paired[p, q, r] = paired[q, p, r]
    );

% Allow pairing across genders, even if not on someone's list. This handles walk-ins and fills schedules.
constraint
    forall(p, q in PEOPLE, r in ROUNDS where p < q) (
        paired[p, q, r] ->
            (wants[p, q] \/ wants[q, p] \/ is_male[p] != is_male[q])
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
            sum(p in PEOPLE where attending[p]) (
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
      10000 * sum(p in PEOPLE where attending[p])(bool2int(has_match_1[p]))
    +  1000 * sum(p in PEOPLE where attending[p])(bool2int(has_match_2[p]))
    +   100 * sum(p in PEOPLE where attending[p])(bool2int(has_match_3[p]));


% ------------------------------------------------------------
% 2. HONORED MATCH COVERAGE
%
% Strongly prefer everyone getting at least one person
% from THEIR OWN list.
%
% Then spread second and third honored matches.
% ------------------------------------------------------------

var int: honored_score =
       80 * sum(p in PEOPLE where attending[p])(bool2int(has_honored_1[p]))
    +   8 * sum(p in PEOPLE where attending[p])(bool2int(has_honored_2[p]))
    +   1 * sum(p in PEOPLE where attending[p])(bool2int(has_honored_3[p]));


% ============================================================
% FINAL OBJECTIVE
% ============================================================

solve maximize
    coverage_score
    + honored_score
    + 2 * mutual_matches;



% ============================================================
% SUMMARY STATISTICS
% ============================================================

var int: n_full_3 = sum(p in PEOPLE where attending[p])(bool2int(match_count[p] == 3));
var int: n_full_2 = sum(p in PEOPLE where attending[p])(bool2int(match_count[p] == 2));
var int: n_full_1 = sum(p in PEOPLE where attending[p])(bool2int(match_count[p] == 1));
var int: n_full_0 = sum(p in PEOPLE where attending[p])(bool2int(match_count[p] == 0));

var int: n_honored_3 = sum(p in PEOPLE where attending[p])(bool2int(honored_count[p] == 3));
var int: n_honored_2 = sum(p in PEOPLE where attending[p])(bool2int(honored_count[p] == 2));
var int: n_honored_1 = sum(p in PEOPLE where attending[p])(bool2int(honored_count[p] == 1));
var int: n_honored_0 = sum(p in PEOPLE where attending[p])(bool2int(honored_count[p] == 0));

var int: num_attended = sum(p in PEOPLE where attending[p])(1);
var int: num_attended_men = sum(p in PEOPLE where attending[p])(bool2int(is_male[p]));
var int: num_attended_women = sum(p in PEOPLE where attending[p])(bool2int(not is_male[p]));

var int: total_meetings =
    sum(p, q in PEOPLE, r in ROUNDS where p < q /\ attending[p] /\ attending[q])(bool2int(paired[p, q, r]));

% How many people p *could* be paired with at all (opposite gender, or on a list either direction, and attending).
array[PEOPLE] of var 0..N: eligible_partners;
constraint
    forall(p in PEOPLE) (
        eligible_partners[p] =
            sum(q in PEOPLE where q != p /\ attending[q]) (
                bool2int(wants[p, q] \/ wants[q, p] \/ is_male[p] != is_male[q])
            )
    );

% How many people p personally listed.
array[PEOPLE] of var 0..N: list_size;
constraint
    forall(p in PEOPLE) (
        list_size[p] = sum(q in PEOPLE where q != p)(bool2int(wants[p, q]))
    );

% Of the people p listed, how many are actually attending.
array[PEOPLE] of var 0..N: attending_list_matches;
constraint
    forall(p in PEOPLE) (
        attending_list_matches[p] =
            sum(q in PEOPLE where q != p /\ wants[p, q])(bool2int(attending[q]))
    );


% ============================================================
% OUTPUT
% ============================================================

output [
    "Total attendees: ", show(num_attended), ": ", show(num_attended_men), " men and ", show(num_attended_women), " women\n\n",
    "How many people attended exactly:\n",
    "  3 rounds: ", show(n_full_3), " / ", show(num_attended),
        " (", show(round(100.0 * n_full_3 / num_attended)), "%)\n",
    "  2 rounds: ", show(n_full_2), " / ", show(num_attended),
        " (", show(round(100.0 * n_full_2 / num_attended)), "%)\n",
    "  1 round:  ", show(n_full_1), " / ", show(num_attended),
        " (", show(round(100.0 * n_full_1 / num_attended)), "%)\n",
    "  0 rounds: ", show(n_full_0), " / ", show(num_attended),
        " (", show(round(100.0 * n_full_0 / num_attended)), "%)\n\n",

    "How many people met with exactly:\n",
    "  3 from list: ", show(n_honored_3), " / ", show(num_attended),
        " (", show(round(100.0 * n_honored_3 / num_attended)), "%)\n",
    "  2 from list: ", show(n_honored_2), " / ", show(num_attended),
        " (", show(round(100.0 * n_honored_2 / num_attended)), "%)\n",
    "  1 from list: ", show(n_honored_1), " / ", show(num_attended),
        " (", show(round(100.0 * n_honored_1 / num_attended)), "%)\n",
    "  0 from list: ", show(n_honored_0), " / ", show(num_attended),
        " (", show(round(100.0 * n_honored_0 / num_attended)), "%)\n\n",

    "How many meetings were mutual: ", show(mutual_matches), " / ", show(total_meetings),
        " (", show(round(100.0 * mutual_matches / total_meetings)), "%)\n\n",

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
             elseif wants[q, p]
             then " (q chose p)"
             else " (neither chose)"
             endif) ++
            "\n"
            | p, q in PEOPLE where p < q /\ fix(paired[p,q,r])
        ])
        | r in ROUNDS
    ]),

    "\nFinal summary:\n",

    concat([
        "\(names[p]): attended " ++ show(match_count[p]) ++ "/" ++ show(R) ++ " rounds" ++
        (if fix(match_count[p]) < R then
            if fix(eligible_partners[p]) < R then
                " [only " ++ show(eligible_partners[p]) ++ " eligible partners in the room]"
            else
                " [eligible partners were already paired with others]"
            endif
        else "" endif) ++
        ", " ++ show(honored_count[p]) ++ " with someone on " ++
        (if is_male[p] then "his" else "her" endif) ++ " list" ++
        (if fix(honored_count[p]) < 3 then
            if fix(list_size[p]) == 0 then
                " [submitted no list]"
            elseif fix(list_size[p]) < 3 then
                " [only listed " ++ show(list_size[p]) ++ " people]"
            elseif fix(honored_count[p]) == fix(attending_list_matches[p]) /\ fix(attending_list_matches[p]) < 3 then
                " [only " ++ show(attending_list_matches[p]) ++ " of " ++ (if is_male[p] then "his" else "her" endif) ++ " list attended]"
            else
                " [listed matches were paired with others]"
            endif
        else "" endif) ++
        "\n"
        | p in PEOPLE where attending[p]
    ])
];
