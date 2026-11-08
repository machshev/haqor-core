#!/usr/bin/env bash
set -euo pipefail

# Fetch local reference copies, without adding the scans or OCR to Git.
# Usage: scripts/fetch-reference-scans.sh [DESTINATION]
# Requires curl, jq and sha256sum; pdftotext is optional (Krahmalkov text).

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly repo_root
readonly manifest="${repo_root}/src_texts/ReferenceScans/files.json"
readonly destination="${1:-${repo_root}/src_texts/ReferenceScans}"

temporary=""
trap 'if [[ -n "$temporary" ]]; then rm -f -- "$temporary"; fi' EXIT

downloads="$(jq -er '.files[] | [.path, .url, .sha256] | @tsv' "$manifest")"
while IFS=$'\t' read -r path url checksum; do
  target="${destination}/${path}"
  if [[ -f "$target" ]] && printf '%s  %s\n' "$checksum" "$target" | sha256sum -c --quiet 2>/dev/null; then
    printf 'Already verified: %s\n' "$path"
    continue
  fi
  mkdir -p -- "$(dirname -- "$target")"
  temporary="$(mktemp "${target}.XXXXXX")"
  curl -fsSL --retry 3 --connect-timeout 20 -o "$temporary" "$url"
  printf '%s  %s\n' "$checksum" "$temporary" | sha256sum -c --quiet
  mv -- "$temporary" "$target"
  temporary=""
  printf 'Fetched and verified: %s\n' "$path"
done <<< "$downloads"

# This PDF has a text layer; the Archive.org works supply their own OCR.
if command -v pdftotext >/dev/null 2>&1; then
  temporary="$(mktemp "${destination}/krahmalkov-2000/source.txt.XXXXXX")"
  pdftotext -layout "${destination}/krahmalkov-2000/source.pdf" "$temporary"
  mv -- "$temporary" "${destination}/krahmalkov-2000/source.txt"
  temporary=""
else
  printf '%s\n' 'Install Poppler (pdftotext) and rerun to extract Krahmalkov search text.'
fi
