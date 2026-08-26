set -e
cargo build --release --quiet
./target/release/matchmaking '/Users/ephraimkunz/Downloads/Mid-Singles Compatibility Questionnaire.csv' > out.txt
python3 parse.py out.txt people.json
node generate.js
open matches.docx
rm out.txt people.json
