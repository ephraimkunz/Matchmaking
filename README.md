# Matchmaking
Generate shortlists of compatible dating partners, based on input dating questionnaire.

## Full flow
1. Create a new dating questionnaire for your group. Visit https://docs.google.com/forms/d/1EbsN5rbwfCNrvdisgqDrTro6-_CVJKSv3AmnOOR17JM/template/preview and click "Use Template" in the upper right.
2. Change the title to remove the word "[Template]".
3. Click Publish and share the link with those taking the survey. Do not change question values, ordering, or add new questions without updating the code in this repo to handle the new format and / or ordering. This repo is tightly coupled with the survey as it needs to read the responses and do a bunch of computation on them.
4. Once you are ready to do matchmaking, download the responses as a CSV file. Ensure the CSV file is unzipped on your computer and remove the zipped file.
6. Download the matchmaking script for your operating system at TODO

## Matchmaking development
Follow steps 1-4 above. Then:
1. Install git if necessary
2. Install rust: https://rust-lang.org/tools/install/
3. Clone this repo: `git clone https://github.com/ephraimkunz/Matchmaking.git`
4. `cd MatchMaking` then `cargo run -- --help` to build the project, run it, and see the available options.
5. A full run against some data might look like `cargo run -- --sort-shortlists-by-score --print-scores <path to downloaded responses CSV>`
6. `cargo test` runs tests. `cargo fmt` runs code formatter. `cargo clippy` runs linter.
7. After pushing changes to Github, create a release and the publish_release Github workflow will automatically kickoff builds for macOS, Windows, and Linux and attach the built artifacts to the release.

