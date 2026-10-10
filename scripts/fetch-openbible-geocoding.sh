#!/usr/bin/env bash
set -euo pipefail

# Update the vendored OpenBible.info Bible geocoding data (CC BY 4.0; its
# OpenStreetMap geometry ODbL 1.0) in src_texts/OpenBible-Geocoding: where
# each place named in the Bible is thought to have been, with how confident
# scholarship is in each identification, and the shapes of its regions,
# rivers and lakes.
#
# `haqor db prepare openbible-geocoding` keeps only the positions and shapes
# from its ancient places file and geometry (places.tsv), and that is what is
# committed. This script fetches the dataset at a pinned commit (git checks
# every file against the commit's hash), verifies the ancient places file's
# checksum and prepares it; review the diff before committing.
readonly commit="7eb18a5ee62f27b9b93bd6689ea272d76dd23b8f"
readonly destination="${1:-src_texts/OpenBible-Geocoding}"
readonly repository="https://github.com/openbibleinfo/Bible-Geocoding-Data"
readonly checksum="b8187aa4737e8517ccc090f765d2be11da4c548cd2a59d3cdcb62e952cb8c0f2"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

git -C "${work}" init --quiet checkout
git -C "${work}/checkout" fetch --quiet --depth 1 "${repository}" "${commit}"
git -C "${work}/checkout" checkout --quiet FETCH_HEAD
printf '%s  %s\n' "${checksum}" "${work}/checkout/data/ancient.jsonl" | sha256sum --check --quiet -

mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db prepare openbible-geocoding \
  --from "${work}/checkout" --to "${destination}"
printf '%s\n%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
  "prepared by haqor db prepare openbible-geocoding from data/ancient.jsonl (sha256 ${checksum}), data/geometry.jsonl and geometry/" \
  > "${destination}/SOURCE"
echo "Prepared OpenBible.info geocoding ${commit} into ${destination}"
