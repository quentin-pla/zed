#!/usr/bin/env zsh
# Import Microsoft FluentUI System Icons (20px regular) as drop-in replacements
# for Zed's UI icon set. Reads script/fluent-icon-map.json for Zed -> FluentUI
# name mapping + skip list. Source: github.com/microsoft/fluentui-system-icons (MIT).
#
# Usage: zsh script/import-fluent-icons.sh
# Deps:  jq, git, find, sed

set -eu
set -o pipefail

REPO_ROOT="${0:A:h:h}"
MAP_FILE="$REPO_ROOT/script/fluent-icon-map.json"
ICONS_DIR="$REPO_ROOT/assets/icons"
UNMATCHED_LOG="$REPO_ROOT/script/fluent-icon-unmatched.txt"
FLUENT_CLONE="/tmp/fluentui-system-icons"
FLUENT_ASSETS="$FLUENT_CLONE/assets"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq not installed. brew install jq" >&2
  exit 1
fi

if [[ ! -f "$MAP_FILE" ]]; then
  echo "ERROR: mapping file missing: $MAP_FILE" >&2
  exit 1
fi

echo "==> Cloning microsoft/fluentui-system-icons (shallow)..."
if [[ -d "$FLUENT_CLONE/.git" ]]; then
  echo "    Existing clone found, pulling latest"
  git -C "$FLUENT_CLONE" pull --depth=1 --quiet
else
  rm -rf "$FLUENT_CLONE"
  git clone --depth=1 --quiet https://github.com/microsoft/fluentui-system-icons.git "$FLUENT_CLONE"
fi

if [[ ! -d "$FLUENT_ASSETS" ]]; then
  echo "ERROR: FluentUI assets dir not found at $FLUENT_ASSETS" >&2
  exit 1
fi

: > "$UNMATCHED_LOG"

replaced=0
missing=0
skipped=0

echo "==> Processing skip list..."
while IFS= read -r zed_name; do
  skipped=$((skipped + 1))
done < <(jq -r '.skip[]' "$MAP_FILE")
echo "    Skipped: $skipped brand/vendor icons (kept original)"

echo "==> Importing FluentUI SVGs..."
while IFS=$'\t' read -r zed_name fluent_name; do
  [[ -z "$zed_name" || -z "$fluent_name" ]] && continue

  src=$(find "$FLUENT_ASSETS" -type f -name "ic_fluent_${fluent_name}_20_regular.svg" -print -quit 2>/dev/null || true)

  if [[ -z "$src" || ! -f "$src" ]]; then
    echo "  MISSING: $zed_name -> ic_fluent_${fluent_name}_20_regular.svg" >&2
    echo "$zed_name -> $fluent_name" >> "$UNMATCHED_LOG"
    missing=$((missing + 1))
    continue
  fi

  dest="$ICONS_DIR/${zed_name}.svg"

  # Normalize: hardcoded #212121 fill/stroke -> currentColor, strip title/desc
  sed -E \
    -e 's/fill="#212121"/fill="currentColor"/g' \
    -e 's/stroke="#212121"/stroke="currentColor"/g' \
    -e '/<title>.*<\/title>/d' \
    -e '/<desc>.*<\/desc>/d' \
    "$src" > "$dest"

  replaced=$((replaced + 1))
done < <(jq -r '.map | to_entries[] | "\(.key)\t\(.value)"' "$MAP_FILE")

echo ""
echo "==> Report"
echo "    Replaced: $replaced"
echo "    Skipped:  $skipped  (brand/vendor — kept original)"
echo "    Missing:  $missing  (logged to $UNMATCHED_LOG)"
echo ""

if [[ "$missing" -gt 0 ]]; then
  echo "Unmatched icons (review + adjust map):"
  cat "$UNMATCHED_LOG" >&2
fi
