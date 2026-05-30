# Matchmaking
Generate shortlists of compatible dating partners, based on input dating questionnaire.

## Full flow
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

## Matchmaking development
Follow steps 1-4 above. Then:
1. Install git if necessary
2. Install rust: https://rust-lang.org/tools/install/
3. Clone this repo: `git clone https://github.com/ephraimkunz/Matchmaking.git`
4. `cd Matchmaking` then `cargo run -- --help` to build the project, run it, and see the available options.
5. A full run against some data might look like `cargo run -- --sort-shortlists-by-score --print-scores <path to downloaded responses CSV>`
6. `cargo test` runs tests. `cargo fmt` runs code formatter. `cargo clippy` runs linter.
7. After pushing changes to Github, create a release and the publish_release Github workflow will automatically kickoff builds for macOS, Windows, and Linux and attach the built artifacts to the release.

### Profiling performance
1. `cargo build --profile profiling`
2. `samply record ./target/profiling/matchmaking <path to csv>`

