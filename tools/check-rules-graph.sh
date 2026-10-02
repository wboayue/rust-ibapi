#!/usr/bin/env bash
#
# Validates the docs/rules/ knowledge graph and its index in AGENTS.md.
#
#   1. every markdown link in AGENTS.md and docs/rules/** resolves
#      to a real file
#   2. every node's frontmatter `id` matches its filename stem
#   3. every `related:` id names an existing node
#   4. AGENTS.md contains no `@`-imports (they would inline every node into
#      context on every session, defeating the point of the index)
#   5. no node cites a `file.rs:NNN` line number — line refs rot silently the
#      same way the retired rule numbers did, and nothing recomputes them. Name
#      the fn, test, or type instead.
#
# `memory:` entries are deliberately NOT validated — that directory lives
# outside the repo and is specific to one maintainer.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RULES_DIR="docs/rules"
INDEX="AGENTS.md"
ERRORS="$(mktemp)"
trap 'rm -f "$ERRORS"' EXIT

nodes() { find "$RULES_DIR" -name '*.md' ! -name 'README.md' | sort; }

# Extract a scalar field from the frontmatter block.
frontmatter_field() {
    sed -n '2,/^---$/p' "$1" | sed -n "s/^$2:[[:space:]]*//p" | head -1
}

# --- 2. id matches filename stem; collect the id set -------------------------

NODE_IDS=""
for file in $(nodes); do
    stem="$(basename "$file" .md)"
    id="$(frontmatter_field "$file" id)"

    if [ -z "$id" ]; then
        echo "$file: no 'id' in frontmatter" >>"$ERRORS"
        continue
    fi
    if [ "$id" != "$stem" ]; then
        echo "$file: id '$id' does not match filename stem '$stem'" >>"$ERRORS"
    fi
    NODE_IDS="$NODE_IDS $id"
done

# --- 3. related: ids resolve -------------------------------------------------

for file in $(nodes); do
    related="$(frontmatter_field "$file" related)"
    [ -z "$related" ] && continue

    # `related: [a, b, c]` -> one id per line
    echo "$related" | tr -d '[]' | tr ',' '\n' | while read -r ref; do
        ref="$(echo "$ref" | tr -d '[:space:]')"
        [ -z "$ref" ] && continue
        case " $NODE_IDS " in
            *" $ref "*) ;;
            *) echo "$file: related id '$ref' names no node" >>"$ERRORS" ;;
        esac
    done
done

# --- 1. markdown links resolve ----------------------------------------------

for file in $INDEX $(nodes) "$RULES_DIR/README.md"; do
    dir="$(dirname "$file")"
    grep -o '](\([^) ]*\))' "$file" 2>/dev/null | sed 's/^](//; s/)$//' | while read -r link; do
        case "$link" in
            http://* | https://* | '#'*) continue ;;
        esac
        target="${link%%#*}"
        [ -z "$target" ] && continue
        [ -e "$dir/$target" ] || echo "$file: dead link -> $link" >>"$ERRORS"
    done
done

# --- 4. no @-imports in the index -------------------------------------------

if grep -nE '^[[:space:]]*@[A-Za-z0-9_./-]+' "$INDEX" >/dev/null 2>&1; then
    grep -nE '^[[:space:]]*@[A-Za-z0-9_./-]+' "$INDEX" |
        sed "s|^|$INDEX: @-import would inline this file into every session -> line |" >>"$ERRORS"
fi

# --- 5. no line-number citations in nodes -----------------------------------
#
# `src/foo.rs:312` is correct until the next edit to foo.rs, and nothing here can
# recompute it. Name the fn / test / type and let the reader grep.

for file in $(nodes); do
    grep -nE '[A-Za-z0-9_/.-]+\.(rs|sh|toml|md):[0-9]+' "$file" |
        sed "s|^|$file: line-number citation (name the fn or test instead) -> line |" >>"$ERRORS"
done

# --- report -----------------------------------------------------------------

count="$(wc -l <"$ERRORS" | tr -d '[:space:]')"
if [ "$count" -gt 0 ]; then
    echo "rules-graph: $count problem(s)"
    sed 's/^/  /' "$ERRORS"
    exit 1
fi

echo "rules-graph: $(nodes | wc -l | tr -d '[:space:]') nodes, all links and edges resolve"
