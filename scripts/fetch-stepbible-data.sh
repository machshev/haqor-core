#!/usr/bin/env bash
set -euo pipefail

# TAHOT is intentionally downloaded from its canonical repository rather than
# redistributed here. Pinning the upstream commit and checksums keeps database
# generation reproducible while following STEP Bible's distribution guidance.
readonly commit="b86d26cdb1f51729e73b5b4eb7f7ccadc5dfba39"
readonly subdir="Translators Amalgamated OT+NT"
readonly destination="${1:-src_texts/STEPBible-Data/${subdir}}"
readonly base_url="https://raw.githubusercontent.com/STEPBible/STEPBible-Data/${commit}/${subdir// /%20}"

readonly files=(
  "TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
  "TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"
)
readonly checksums=(
  "e9b8546ee48fe0bfc57c3b70f5f40e98d96580e803526d19026224e31753368b"
  "f3ded203d2a74d6368932c97ae550d1d0754b271af491dc0dedf36fe3ba0bcc5"
  "84e118a97e5725e3847cdfdd593873513021c790c63cc91a0d41fca2b5db2ed5"
  "195fee1dc3653bab33701f170734eb894ed647c10cd08cc61749375fe8b73775"
)

mkdir -p "${destination}"
for index in "${!files[@]}"; do
  file="${files[index]}"
  target="${destination}/${file}"
  temporary="${target}.part"
  encoded_file="${file// /%20}"
  curl -L --fail --retry 3 --output "${temporary}" "${base_url}/${encoded_file}"
  printf '%s  %s\n' "${checksums[index]}" "${temporary}" | sha256sum --check -
  mv "${temporary}" "${target}"
done

echo "Fetched pinned STEP Bible TAHOT data into ${destination}"
