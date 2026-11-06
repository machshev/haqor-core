#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Proto-Semitic root derivations in
# src_texts/ProtoSemitic from Zenodo record 14160276 (Anderson and Moser,
# 2024): each row links a Proto-Semitic root to an entry in Akkadian, Arabic,
# Aramaic, Ge'ez, Hebrew, Punic or Ugaritic. The file is checked in as
# published; its MD5 is the one Zenodo states.
#
# Usage: scripts/fetch-protosemitic-roots.sh [DESTINATION]

readonly record="https://zenodo.org/records/14160276"
readonly file="SemiticRootDerivations.csv"
readonly md5="1aceec93c7b11aa444eb99725270e7ca"
readonly destination="${1:-src_texts/ProtoSemitic}"

mkdir -p "${destination}"
curl -sS -L --fail --retry 3 -o "${destination}/${file}" "${record}/files/${file}?download=1"
echo "${md5}  ${destination}/${file}" | md5sum -c --quiet

{
  echo "Source: ${record}"
  echo "File: ${file}"
  echo "MD5: ${md5}"
} > "${destination}/SOURCE"
