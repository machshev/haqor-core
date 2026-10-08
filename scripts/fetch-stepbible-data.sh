#!/usr/bin/env bash
set -euo pipefail

# Update the vendored STEP Bible data in src_texts/STEPBible-Data.
#
# - TAHOT: the Hebrew OT tagged word by word, with a contextual translation.
# - TIPNR: every person and place named in the Bible, told apart.
# - TBESH: the brief lexicon of the Hebrew words' senses.
#
# The three are joined by Strong's numbers, which Haqor does not keep: `haqor
# db prepare stepbible` joins them and writes what the build reads (tahot.tsv,
# senses.tsv, names.jsonl), and that is what is committed. This script
# downloads the published files at a pinned commit, verifies their checksums
# and prepares them; review the diff before committing.
readonly commit="b86d26cdb1f51729e73b5b4eb7f7ccadc5dfba39"
readonly destination="${1:-src_texts/STEPBible-Data}"
readonly base_url="https://raw.githubusercontent.com/STEPBible/STEPBible-Data/${commit}"

readonly files=(
  "Translators Amalgamated OT+NT/TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "Translators Amalgamated OT+NT/TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "Translators Amalgamated OT+NT/TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "Translators Amalgamated OT+NT/TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "Proper Nouns/TIPNR - Translators Individualised Proper Names with all References - STEPBible.org CC BY.txt"
  "Lexicons/TBESH - Translators Brief lexicon of Extended Strongs for Hebrew - STEPBible.org CC BY.txt"
)
readonly checksums=(
  "e9b8546ee48fe0bfc57c3b70f5f40e98d96580e803526d19026224e31753368b"
  "f3ded203d2a74d6368932c97ae550d1d0754b271af491dc0dedf36fe3ba0bcc5"
  "84e118a97e5725e3847cdfdd593873513021c790c63cc91a0d41fca2b5db2ed5"
  "195fee1dc3653bab33701f170734eb894ed647c10cd08cc61749375fe8b73775"
  "1a3b7d7df5cfa1e96eefa07dec92900bea278370c6788fadb5d036f3223b637c"
  "464dccadd95fd8620dd05fa0d7a4caba58ec3c4d5db3ebf38e43d046ca25b591"
)

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

for index in "${!files[@]}"; do
  file="${files[index]}"
  target="${work}/${file}"
  mkdir -p "$(dirname "${target}")"
  curl -sS -L --fail --retry 3 --output "${target}" "${base_url}/${file// /%20}"
  printf '%s  %s\n' "${checksums[index]}" "${target}" | sha256sum --check --quiet -
done

mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db prepare stepbible \
  --from "${work}" --to "${destination}"
{
  printf '%s\n%s\n' "repository: https://github.com/STEPBible/STEPBible-Data" "commit: ${commit}"
  printf 'prepared by haqor db prepare stepbible from:\n'
  for index in "${!files[@]}"; do
    printf '  %s  %s\n' "${checksums[index]}" "${files[index]}"
  done
} > "${destination}/SOURCE"
echo "Prepared STEP Bible TAHOT, TIPNR and TBESH ${commit} into ${destination}"
