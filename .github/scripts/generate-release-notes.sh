#!/usr/bin/env bash
set -euo pipefail

current_tag="${1:-${GITHUB_REF_NAME:-}}"
output_file="${2:-release-notes.md}"

if [[ -z "$current_tag" ]]; then
  echo "usage: $0 <tag> [output-file]" >&2
  exit 1
fi

if ! git rev-parse -q --verify "refs/tags/$current_tag" >/dev/null; then
  echo "tag not found: $current_tag" >&2
  exit 1
fi

previous_tag="$(
  git tag \
    --merged "$current_tag" \
    --list 'v[0-9]*.[0-9]*.[0-9]*' \
    --sort=-v:refname \
    | grep -vxF "$current_tag" \
    | head -n 1 \
    || true
)"

{
  echo "# ${current_tag}"
  echo

  if [[ -n "$previous_tag" ]]; then
    echo "Changes since ${previous_tag}."
    echo
    echo "Compare: https://github.com/${GITHUB_REPOSITORY:-Feel-ix-343/markdown-oxide}/compare/${previous_tag}...${current_tag}"
    echo
    range="${previous_tag}..${current_tag}"
  else
    echo "Initial release."
    echo
    range="$current_tag"
  fi

  echo "## Changes"
  echo

  notes="$(
    git log \
      --no-merges \
      --reverse \
      --format='- %s (%h)' \
      "$range" \
      | grep -Ev '^- (version bump|chore: bump version|bump version)( \(| to )' \
      || true
  )"

  if [[ -n "$notes" ]]; then
    echo "$notes"
  else
    echo "- Version bump only."
  fi
} > "$output_file"

echo "Generated release notes at $output_file"
