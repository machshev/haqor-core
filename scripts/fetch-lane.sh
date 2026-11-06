#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Lane's Lexicon in src_texts/Lane from the laneslexicon
# project's corrected copy of the Perseus TEI (its `master` branch; the
# untouched Perseus files are on its `originals` branch).
#
# The XML is 81 MB in Buckwalter transliteration; `db import-lane` turns it
# into one JSON line per entry with the Arabic in Unicode, which is what is
# checked in. The commit is pinned so a refresh is deliberate: bump it and
# review the diff like any other source change.
#
# Usage: scripts/fetch-lane.sh [DESTINATION]

readonly repo="https://github.com/laneslexicon/lexicon_xml.git"
readonly commit="f3c19fb29f2cf2e12de3f97b7ce2b7a0d6682ea6"
readonly destination="${1:-src_texts/Lane}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

git -C "${work}" init -q
git -C "${work}" fetch -q --depth 1 "${repo}" "${commit}"
git -C "${work}" checkout -q FETCH_HEAD

cargo run --quiet --release -- db import-lane --input "${work}" --output "${destination}"

{
  echo "Source: ${repo}"
  echo "Commit: ${commit}"
  echo "Upstream: Perseus Digital Library, Lane's Arabic-English Lexicon TEI XML"
} > "${destination}/SOURCE"
