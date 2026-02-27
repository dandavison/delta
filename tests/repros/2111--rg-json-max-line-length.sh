#!/bin/bash
# Run: bash tests/repro_2111.sh [path-to-delta]
set -euo pipefail

DELTA="${1:-target/debug/delta}"
MAX=100

long_code="var x = $(python3 -c "print('a' * 5000)");"
rg_json=$(python3 -c "
import json, sys
code = sys.argv[1]
print(json.dumps({'type': 'begin', 'data': {'path': {'text': 'test.js'}}}))
print(json.dumps({'type': 'match', 'data': {
    'path': {'text': 'test.js'},
    'lines': {'text': code + chr(10)},
    'line_number': 1, 'absolute_offset': 0,
    'submatches': [{'match': {'text': 'var'}, 'start': 0, 'end': 3}]
}}))
print(json.dumps({'type': 'end', 'data': {
    'path': {'text': 'test.js'}, 'binary_offset': None,
    'stats': {'elapsed': {'secs':0,'nanos':100,'human':'0s'},
              'searches':1,'searches_with_match':1,
              'bytes_searched':100,'bytes_printed':100,
              'matched_lines':1,'matches':1}
}}))
print(json.dumps({'type': 'summary', 'data': {
    'elapsed_total': {'secs':0,'nanos':100,'human':'0s'},
    'stats': {'bytes_printed':100,'bytes_searched':100,
              'elapsed':{'human':'0s','nanos':100,'secs':0},
              'matched_lines':1,'matches':1,'searches':1,
              'searches_with_match':1}
}}))
" "$long_code")

output=$(echo "$rg_json" | GIT_CONFIG_NOSYSTEM=1 HOME=/nonexistent "$DELTA" --max-line-length "$MAX" 2>&1) || true
max_width=0
while IFS= read -r line; do
    w=$(echo -n "$line" | perl -pe 's/\e\[[0-9;]*m//g' | wc -m)
    (( w > max_width )) && max_width=$w
done <<< "$output"

cat <<EOF
# Repro: #2111 -- walls of text from minified JS with \`rg --json\`

https://github.com/dandavison/delta/issues/2111

When \`rg --json\` output contains a long code line (e.g. minified JS), delta's
\`--max-line-length\` should truncate it. Previously the long line was displayed
verbatim, producing a wall of text.

## Scenario: 5000-char code line with \`--max-line-length $MAX\`

**Expected:** every output line has visible width <= ~$MAX characters.

**Command:**

\`\`\`bash
echo "\$rg_json" | GIT_CONFIG_NOSYSTEM=1 HOME=/nonexistent $DELTA --max-line-length $MAX
\`\`\`

where \`\$rg_json\` is synthetic \`rg --json\` output containing a ~5000-char JS line.

**Output (first 200 chars of each line):**

\`\`\`
$(echo "$output" | perl -pe 's/\e\[[0-9;]*m//g' | cut -c1-200)
\`\`\`

**Max visible line width:** $max_width characters.

## Findings

EOF

if (( max_width > MAX + 20 )); then
    echo "BUG CONFIRMED: max visible width is $max_width, far exceeding --max-line-length $MAX."
else
    echo "PASS: all lines are within tolerance of --max-line-length $MAX (max width: $max_width)."
fi
