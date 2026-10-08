#!/usr/bin/env bash
set -euo pipefail

# Update the vendored OpenBible.info Bible geocoding data (CC BY 4.0) in
# src_texts/OpenBible-Geocoding: where each place named in the Bible is
# thought to have been, with how confident scholarship is in each
# identification.
#
# `haqor db prepare openbible-geocoding` keeps only the positions from its
# ancient places file (places.tsv), and that is what is committed. This
# script downloads the file at a pinned commit, verifies its checksum and
# prepares it; review the diff before committing.
readonly commit="7eb18a5ee62f27b9b93bd6689ea272d76dd23b8f"
readonly destination="${1:-src_texts/OpenBible-Geocoding}"
readonly repository="https://github.com/openbibleinfo/Bible-Geocoding-Data"
readonly base_url="https://raw.githubusercontent.com/openbibleinfo/Bible-Geocoding-Data/${commit}"
readonly checksum="b8187aa4737e8517ccc090f765d2be11da4c548cd2a59d3cdcb62e952cb8c0f2"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

curl -sS -L --fail --retry 3 --output "${work}/ancient.jsonl" "${base_url}/data/ancient.jsonl"
printf '%s  %s\n' "${checksum}" "${work}/ancient.jsonl" | sha256sum --check --quiet -

mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db prepare openbible-geocoding \
  --from "${work}" --to "${destination}"
printf '%s\n%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
  "prepared by haqor db prepare openbible-geocoding from data/ancient.jsonl (sha256 ${checksum})" \
  > "${destination}/SOURCE"
echo "Prepared OpenBible.info geocoding ${commit} into ${destination}"
