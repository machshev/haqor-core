#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Septuagint alignment in src_texts/MACULA-LXX: for
# each Hebrew word MACULA Hebrew aligns to the Septuagint, its Greek form and
# lemma (ADR 7). Same MACULA commit as scripts/fetch-macula-hebrew.sh. The
# Greek lemmas are named from Open Scriptures' JSON edition of Strong's Greek
# dictionary, so no Strong's number is kept.
#
# Usage: scripts/fetch-macula-lxx.sh [DESTINATION]

readonly repository="https://github.com/Clear-Bible/macula-hebrew"
readonly commit="47db250bd55d0d8577f2a94fba114ef16c35b23c"
readonly strongs_commit="0acd2f251c2d35ff8db2dece4e0593979d3ac223"
readonly strongs_url="https://raw.githubusercontent.com/openscriptures/strongs/${strongs_commit}/greek/strongs-greek-dictionary.js"
readonly strongs_sha256="7624ee738ae47e80f1a352223e28a26d011c9cd4898822cee52f47a010c04efd"
readonly destination="${1:-src_texts/MACULA-LXX}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

git -c advice.detachedHead=false clone --quiet --filter=blob:none --no-checkout \
  "${repository}" "${work}/macula-hebrew"
git -C "${work}/macula-hebrew" sparse-checkout set --no-cone '/WLC/lowfat/*-lowfat.xml'
git -C "${work}/macula-hebrew" -c advice.detachedHead=false checkout --quiet "${commit}"

curl -sS -L --fail --retry 3 -o "${work}/strongs.js" "${strongs_url}"
echo "${strongs_sha256}  ${work}/strongs.js" | sha256sum -c --quiet

mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db import-lxx \
  --lowfat "${work}/macula-hebrew/WLC/lowfat" --strongs "${work}/strongs.js" --output "${destination}"
printf '%s\n' "repository: ${repository}" "commit: ${commit}" \
  "lemmas: ${strongs_url}" \
  "prepared: WLC/lowfat/*-lowfat.xml greek attributes reduced to lxx.tsv by haqor db import-lxx" \
  > "${destination}/SOURCE"
