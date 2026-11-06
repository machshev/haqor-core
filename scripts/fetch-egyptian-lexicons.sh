#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Egyptian and Coptic dictionaries in src_texts/Egyptian,
# for tracing the Hebrew Bible's Egyptian loanwords (ADR 7):
#
# - the AED's list of Egyptian words (`dictionary.xml` of aed-tei, from the
#   Thesaurus Linguae Aegyptiae's January 2018 database extract), and
# - the Comprehensive Coptic Lexicon v1.2 (BBAW and FU Berlin, via KELLIA).
#
# Both are fetched at pinned commits and checked against pinned SHA-256s.
#
# Usage: scripts/fetch-egyptian-lexicons.sh [DESTINATION]

readonly destination="${1:-src_texts/Egyptian}"
readonly aed_commit="656a0ce3dd6af5ff3b1df9f8fcac4bedafa07750"
readonly aed_sha256="faaca47fc4e3c89899a764e56321ca2dc051e58ef6462acedaa6205ea666b730"
readonly coptic_commit="f365e2d239bd12b6897f3a761487e40d384b2b24"
readonly coptic_sha256="ace84fce11b76386a6adb9888b2cb947056954fc9fe9c04aa0676b1f595fd43d"

mkdir -p "${destination}"
fetch() {
  curl -sS -L --fail --retry 3 -o "${destination}/$1" "$2"
  echo "$3  ${destination}/$1" | sha256sum -c --quiet
}
fetch aed-dictionary.xml \
  "https://raw.githubusercontent.com/JKatzwinkel/aed-tei/${aed_commit}/files/dictionary.xml" "${aed_sha256}"
fetch coptic-lexicon.xml \
  "https://raw.githubusercontent.com/KELLIA/dictionary/${coptic_commit}/xml/Comprehensive_Coptic_Lexicon-v1.2-2020.xml" "${coptic_sha256}"

{
  echo "aed-dictionary.xml: https://github.com/JKatzwinkel/aed-tei commit ${aed_commit}"
  echo "coptic-lexicon.xml: https://github.com/KELLIA/dictionary commit ${coptic_commit}"
} > "${destination}/SOURCE"
