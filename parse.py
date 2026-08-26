import re, json, sys

NAME_EMAIL_RE = re.compile(r'^(.*?)\s*\(([^)]*@[^)]*)\)\s*$')

def parse(text):
    blocks = re.split(r'^=+\s*$', text, flags=re.MULTILINE)
    people = []
    for block in blocks:
        lines = [l.rstrip() for l in block.split('\n')]
        lines = [l for l in lines]
        # find first non-blank line -> header
        idx = 0
        while idx < len(lines) and lines[idx].strip() == '':
            idx += 1
        if idx >= len(lines):
            continue
        header = lines[idx].strip()
        m = NAME_EMAIL_RE.match(header)
        if not m:
            continue  # not a real block
        person = {'name': m.group(1).strip(), 'email': m.group(2).strip(), 'matches': []}
        idx += 1
        # skip until "Matches:" line
        while idx < len(lines) and lines[idx].strip().lower() != 'matches:':
            idx += 1
        idx += 1  # skip the "Matches:" line itself

        current_match = None
        for line in lines[idx:]:
            stripped = line.strip()
            if stripped == '':
                continue
            mm = NAME_EMAIL_RE.match(stripped)
            if mm:
                # new match entry
                current_match = {'name': mm.group(1).strip(), 'email': mm.group(2).strip(), 'attrs': []}
                person['matches'].append(current_match)
            elif current_match is not None and ':' in stripped:
                label, value = stripped.split(':', 1)
                current_match['attrs'].append({'label': label.strip(), 'value': value.strip()})
            # else: stray line, ignore
        people.append(person)
    return people

if __name__ == '__main__':
    with open(sys.argv[1], encoding='utf-8') as f:
        text = f.read()
    people = parse(text)
    with open(sys.argv[2], 'w', encoding='utf-8') as f:
        json.dump(people, f, indent=2, ensure_ascii=False)
    print(f"Parsed {len(people)} people, total matches: {sum(len(p['matches']) for p in people)}")
