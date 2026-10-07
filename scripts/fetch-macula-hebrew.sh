#!/usr/bin/env bash
set -euo pipefail

# MACULA Hebrew's syntax trees are fetched from their canonical repository
# rather than redistributed here (about 400 MB of XML). Checking out a pinned
# commit makes git verify every file against that commit's hashes, which keeps
# database generation reproducible.
readonly repository="https://github.com/Clear-Bible/macula-hebrew"
readonly commit="47db250bd55d0d8577f2a94fba114ef16c35b23c"
readonly destination="${1:-src_texts/MACULA-Hebrew}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

git -c advice.detachedHead=false clone --quiet --filter=blob:none --no-checkout \
  "${repository}" "${work}/macula-hebrew"
git -C "${work}/macula-hebrew" sparse-checkout set --no-cone \
  '/WLC/lowfat/*-lowfat.xml' '/LICENSE.md' '/README.md'
git -C "${work}/macula-hebrew" -c advice.detachedHead=false checkout --quiet "${commit}"

rm -rf "${destination}"
mkdir -p "${destination}/lowfat"
# The chapter files only: `macula-hebrew-lowfat.xml` merely XIncludes them.
find "${work}/macula-hebrew/WLC/lowfat" -name '[0-9][0-9]-*-lowfat.xml' \
  -exec cp {} "${destination}/lowfat/" \;
cp "${work}/macula-hebrew/LICENSE.md" "${work}/macula-hebrew/README.md" "${destination}/"
printf '%s\n%s\n' "repository: ${repository}" "commit: ${commit}" > "${destination}/SOURCE"

count="$(find "${destination}/lowfat" -name '*.xml' | wc -l)"
if [[ "${count}" -ne 929 ]]; then
  echo "expected 929 chapter files, found ${count}" >&2
  exit 1
fi
echo "Fetched MACULA Hebrew ${commit} into ${destination}"
