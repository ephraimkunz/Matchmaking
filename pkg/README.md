# Matchmaking
Generate shortlists of compatible dating partners, based on input dating questionnaire.

## Graphical user interace
1. Create a new dating questionnaire for your group. Visit https://docs.google.com/forms/d/1vrJOtTK-0Qxo373gTRxZvXdbvJ-wX03EVaqJu_rWulQ/template/preview and click "Use Template" in the upper right.
2. Change the title to remove the word "[Template]".
3. Click Publish and share the link with those taking the survey. Do not change question values, ordering, or add new questions without updating the code in this repo to handle the new format and / or ordering. This repo is tightly coupled with the survey as it needs to read the responses and do a bunch of computation on them.
4. Once you are ready to do matchmaking, download the responses as a CSV file. Ensure the CSV file is unzipped on your computer and remove the zipped file. If you want a sample file for testing, download one of the sample files in https://github.com/ephraimkunz/Matchmaking/tree/main/test_data.
5. Visit https://ephraimkunz.github.io/Matchmaking/ and use the GUI to upload the CSV file and run matchmaking.

<img width="1256" height="830" alt="gui" src="https://github.com/user-attachments/assets/4ae04aaa-3e24-4b1c-b49a-72cd68102a3e" />


## Command line interace
1. Create a new dating questionnaire for your group. Visit https://docs.google.com/forms/d/1EbsN5rbwfCNrvdisgqDrTro6-_CVJKSv3AmnOOR17JM/template/preview and click "Use Template" in the upper right.
2. Change the title to remove the word "[Template]".
3. Click Publish and share the link with those taking the survey. Do not change question values, ordering, or add new questions without updating the code in this repo to handle the new format and / or ordering. This repo is tightly coupled with the survey as it needs to read the responses and do a bunch of computation on them.
4. Once you are ready to do matchmaking, download the responses as a CSV file. Ensure the CSV file is unzipped on your computer and remove the zipped file.
5. Download the matchmaking script for your operating system at https://github.com/ephraimkunz/Matchmaking/releases. 
6. Unzip the downloaded script and run it from the commandline like `./matchmaking`. You may need to adjust permissions on your machine to let it run an unsigned binary. You can google how to do that on your operating system.
7. Once you can run the matchmaking script from the commandline, you can pass the help flag for options: `./matchmaking -h`. The most basic usage is to pass the path to the CSV downloaded in step 4 to the tool. For example, `./matchmaking ~/Downloads/responses_to_survey.csv`. This will run the match algorithm and print out the shortlist of most compatible matches for each individual. The output will be something like this:

```
Aaliyah Evans (aaliyah.evans@example.com)

Matches:
        Tyler Collins (tyler.collins@example.com)
        Niche interest most people don't know I have: Beekeeping
        Something I've changed my mind about recently: Whether I want to live in a big city
        The thing I find most attractive in a person: Passion for something they care about

        Paxton Henderson (paxton.henderson@example.com)
        My weekend usually looks like: Working on my hobbies
        Niche interest most people don't know I have: Amateur astronomy
        Something I'm better at than I let on: Parallel parking

        Caleb Young (caleb.young@example.com)
        Something I'm better at than I let on: Keeping plants alive
        The thing I find most attractive in a person: The ability to be silly and serious
        Unpopular opinion I stand by: College is not worth it for everyone

========================================================================

Aaron Phillips (aaron.phillips@example.com)

Matches:
        Isabella Jones (isabella.jones@example.com)
        Unpopular opinion I stand by: Hiking is just walking
        The thing I find most attractive in a person: Being present in conversation
        My weekend usually looks like: Working on a project at home

        Emily Jackson (emily.jackson@example.com)
        The thing I find most attractive in a person: Being low-maintenance
        Something I've changed my mind about recently: The importance of budgeting
        Unpopular opinion I stand by: Hiking is just walking

        Emma Johnson (emma.johnson@example.com)
        My weekend usually looks like: Church, family dinner, and relaxing
        Ideal low-effort hangout: Cooking something new
        Something I've changed my mind about recently: How I feel about early mornings

========================================================================

Abigail Thomas (abigail.thomas@example.com)

Matches:
        Spencer Morris (spencer.morris@example.com)
        Ideal low-effort hangout: Movie at home with snacks
        Niche interest most people don't know I have: Antique book collecting
        Something I'm better at than I let on: Memorizing lyrics

        Bryce Torres (bryce.torres@example.com)
        Unpopular opinion I stand by: Breakfast food is overrated
        I could give a 10-minute talk on: How to fix almost anything
        Something I'm better at than I let on: Managing money
```
By default the order of the shortlist for a person will be a random shuffling of their top matches, and the scores will not be printed. They can be shuffled by score and the scores can be printed by toggling commandline options.

An important command-line option is `-d`, which prints diagnostics and will see how high of quality a run is and what tuning it might need:
```
POOL  is the input usable?
  male_count                  50
  female_count                50
  pairs_scored               944
  dealbreaker_eliminated    1556
    wants_children            90
    stay_local               272
    marriage_timeline        131
    religion                1063

  pairs_scored: (male, female) pairs that survived all dealbreakers and got a score
  dealbreaker_eliminated: pairs rejected before scoring due to dealbreakers

CONVERGENCE  did the algorithm finish cleanly?
  cap_relaxed             true
  appearance_max            10
  appearance_stddev       2.75
  zero_appearances           7

  shortlist lengths
    0 (no matches)  |                                                     0
    1               |                                                     1
    2               | #                                                   2
    3               | ##                                                  4
    4               | ####                                                8
    5 (full)        | ##################################################  85

  cap_relaxed: true if the appearance cap had to be raised to make progress; true = pool was tight and quality may have suffered
  appearance_max: the most times any one person was picked; should sit at the cap when the pool is tight
  appearance_stddev: spread of pick counts; low = even distribution, high = a few popular people absorbed many picks while others got none
  zero_appearances: people no one's shortlist included; see histogram index 0 for the subject-side complement
  shortlist lengths: exact count of people with each shortlist length; 0 = no matches, last bucket = full target

QUALITY  is the output good?
  rank_regret_mean      0.34
  rank_regret_p95          2
  mutual_rate          65.8%

  score distribution
    0.553-0.562     | ###                                                 4
    0.562-0.571     |                                                     1
    0.571-0.579     | #                                                   2
    0.579-0.588     | #####                                               7
    0.588-0.597     | ##########                                          13
    0.597-0.605     | ##########                                          13
    0.605-0.614     | ############                                        16
    0.614-0.623     | ################################                    41
    0.623-0.632     | #################################                   42
    0.632-0.640     | ############################################        56
    0.640-0.649     | ##################################################  63
    0.649-0.658     | ######################################              49
    0.658-0.666     | ############################################        56
    0.666-0.675     | ##################################                  44
    0.675-0.684     | ##################                                  23
    0.684-0.693     | ###################                                 24
    0.693-0.701     | #########                                           12
    0.701-0.710     | ###                                                 4
    0.710-0.719     |                                                     0
    0.719-0.727     | ###                                                 4

  rank_regret_mean: extra candidates skipped per pick because higher-ranked options were at the appearance cap. 0 = every pick was the best still-available match; 2 = on average the cap forced 2 better candidates to be skipped before each pick. Larger means the cap is biting harder.
  rank_regret_p95: same skip-count, 95th percentile. A small mean with a large p95 means most picks were unblocked but a few people had popular candidates capped out and got pushed deep into their list.
  mutual_rate: fraction of shortlist entries where B is also on A's list; 100% = every match is mutual, low values mean many one-sided introductions
  score distribution: distribution of scores that were actually served, auto-ranged to the observed [min, max]; mass in high buckets is healthy, weight in low buckets means someone got a poor match
```

## Matchmaking development
Follow steps 1-4 above. Then:
1. Install git if necessary
2. Install rust: https://rust-lang.org/tools/install/
3. Clone this repo: `git clone https://github.com/ephraimkunz/Matchmaking.git`
4. `cd Matchmaking` then `cargo run -- --help` to build the project, run it, and see the available options.
5. A full run against some data might look like `cargo run -- --sort-shortlists-by-score --print-scores <path to downloaded responses CSV>`
6. `cargo test` runs tests. `cargo fmt` runs code formatter. `cargo clippy` runs linter.
7. After pushing changes to Github, create a release and the publish_release Github workflow will automatically kickoff builds for macOS, Windows, and Linux and attach the built artifacts to the release.
8. Pushing to Github will build and deploy the GUI onto this repo's Github pages. This GUI is a static website that uses WebAssembly to run the matchmaking code in the browser.<img width="1256" height="830" alt="gui" src="https://github.com/user-attachments/assets/45907d62-e78c-4d30-ae67-7cf7c2b65128" />


### Profiling performance
1. `cargo build --profile profiling`
2. `samply record ./target/profiling/matchmaking <path to csv>`

