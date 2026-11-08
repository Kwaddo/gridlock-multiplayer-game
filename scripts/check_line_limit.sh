#!/usr/bin/env bash
# Fails when a tracked source file is longer than the line limit (500 by
# default; LINE_LIMIT overrides it). Split long files into modules by
# responsibility, with constants in their own module, instead of raising it.
#
#   ./scripts/check_line_limit.sh
set -euo pipefail

LIMIT="${LINE_LIMIT:-500}"
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Files that must stay whole. registry/dcs/index.js is deployed as one inline
# file (DCS deploy_app), so it cannot be split into modules.
EXEMPT=(
    "registry/dcs/index.js"
)

status=0
while IFS= read -r file; do
    for exempt in "${EXEMPT[@]}"; do
        [ "$file" = "$exempt" ] && continue 2
    done
    [ -f "$file" ] || continue
    lines=$(wc -l < "$file" | tr -d ' ')
    if [ "$lines" -gt "$LIMIT" ]; then
        echo "$file: $lines lines (limit $LIMIT)"
        status=1
    fi
done < <(git ls-files -co --exclude-standard -- '*.rs' '*.js' '*.mjs' '*.sh' ':!:web/mq_js_bundle.js')

if [ "$status" -eq 0 ]; then
    echo "All source files are within $LIMIT lines."
fi
exit "$status"
