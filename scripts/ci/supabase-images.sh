#!/usr/bin/env bash
# The container images the `cloud-rls` job runs (RLS pgTAP plan, D2). Always invoked as
# `bash scripts/ci/supabase-images.sh <command>` from the repository root: a file committed from
# Windows lands as 100644, so a bare path would fail with "Permission denied" (r1 I5).
#
#   baseline  save the images already on the runner, before the CLI pulls anything
#   list      print every image that is new since the baseline
#   pull      pull each image in cloud/supabase/tests/images.lock by digest and tag it, so the CLI
#             finds it locally and pulls nothing (T4 wires this in)
#   verify    fail when an image new since the baseline is not in the lock, or a locked tag points
#             at another digest (T4 wires this in)
#
# One image per line, `repo:tag@sha256:<digest>`, the shape the lock uses. Names and digests only:
# nothing here prints a credential.
set -euo pipefail

lock="cloud/supabase/tests/images.lock"
dir="${RUNNER_TEMP:?RUNNER_TEMP is not set}"
baseline="$dir/supabase-images.baseline"

images() {
  docker image ls --digests --format '{{.Repository}}:{{.Tag}}@{{.Digest}}' | LC_ALL=C sort -u
}

new_since_baseline() {
  if [ ! -f "$baseline" ]; then
    echo "supabase-images: no baseline at $baseline; run 'baseline' first" >&2
    exit 1
  fi
  local now
  now="$(images)"
  LC_ALL=C comm -13 "$baseline" <(printf '%s\n' "$now")
}

locked() {
  if [ ! -f "$lock" ]; then
    echo "supabase-images: $lock does not exist" >&2
    exit 1
  fi
  local out
  out="$(grep -Ev '^[[:space:]]*(#|$)' "$lock" || true)"
  if [ -z "$out" ]; then
    echo "supabase-images: $lock names no image" >&2
    exit 1
  fi
  printf '%s\n' "$out"
}

# Split `repo:tag@sha256:<digest>` into the globals repo, tag and digest; refuse anything else.
split_ref() {
  local line="$1" ref
  ref="${line%@*}"
  digest="${line##*@}"
  repo="${ref%:*}"
  tag="${ref##*:}"
  if [[ "$line" != *@* || "$ref" != *:* || ! "$digest" =~ ^sha256:[0-9a-f]{64}$ ]]; then
    echo "supabase-images: not repo:tag@sha256:<digest>: $line" >&2
    exit 1
  fi
}

case "${1:-}" in
  baseline)
    images > "$baseline"
    echo "supabase-images: $(grep -c . "$baseline" || true) images already on the runner"
    ;;
  list)
    new="$(new_since_baseline)"
    echo "supabase-images: $(printf '%s' "$new" | grep -c . || true) images new since the baseline"
    if [ -n "$new" ]; then printf '%s\n' "$new"; fi
    ;;
  pull)
    lines="$(locked)"
    while IFS= read -r line; do
      split_ref "$line"
      docker pull --quiet "$repo@$digest"
      docker tag "$repo@$digest" "$repo:$tag"
    done <<< "$lines"
    ;;
  verify)
    lines="$(locked)"
    new="$(new_since_baseline)"
    bad=0
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      if [[ "$line" == *"@<none>" ]]; then
        echo "supabase-images: an image with no digest ran: $line" >&2
        bad=1
        continue
      fi
      split_ref "$line"
      found=0
      while IFS= read -r l; do
        # Exact, or an untagged listing of a locked repo and digest (pulled by digest, then tagged).
        if [[ "$l" == "$line" || ( "$tag" == "<none>" && "$l" == "$repo:"*"@$digest" ) ]]; then found=1; fi
      done <<< "$lines"
      if [ "$found" -eq 0 ]; then
        echo "supabase-images: new since the baseline and not in $lock: $line" >&2
        bad=1
      fi
    done <<< "$new"
    while IFS= read -r line; do
      split_ref "$line"
      if ! docker image inspect --format '{{join .RepoDigests "\n"}}' "$repo:$tag" | grep -qxF "$repo@$digest"; then
        echo "supabase-images: $repo:$tag is not the locked $digest" >&2
        bad=1
      fi
    done <<< "$lines"
    if [ "$bad" -eq 0 ]; then echo "supabase-images: every image that ran is locked"; fi
    exit "$bad"
    ;;
  *)
    echo "usage: bash scripts/ci/supabase-images.sh baseline|list|pull|verify" >&2
    exit 2
    ;;
esac
