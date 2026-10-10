#!/usr/bin/env bash
set -euo pipefail

# Update the vendored English of the Peshitta New Testament, James Murdock's
# translation (1852, public domain), in src_texts/Murdock.
#
# The text is CrossWire's SWORD module of it, exported with mod2imp (from
# SWORD, in the dev shell). `haqor db prepare murdock` reduces the export to
# the verses the build reads (murdock.tsv), putting right the module's
# misnumbered verses as it goes, and that is what is committed. This script
# downloads the module, verifies its checksum and prepares it; review the
# diff before committing.
readonly module_url="https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Murdock.zip"
readonly module_checksum="4f0adeba385acbfa37921f66677d4aaf99e23b4e65ca162f122832689036641f"
readonly destination="${1:-src_texts/Murdock}"

if ! command -v mod2imp >/dev/null; then
  echo "mod2imp not found: run this in the dev shell (nix develop)" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

curl --fail --silent --show-error --location --output "${work}/Murdock.zip" "${module_url}"
echo "${module_checksum}  ${work}/Murdock.zip" | sha256sum --check --quiet
unzip -q "${work}/Murdock.zip" -d "${work}/sword"
SWORD_PATH="${work}/sword" mod2imp Murdock > "${work}/murdock.imp"

rm -rf "${destination}"
mkdir -p "${destination}"
cp "${work}/sword/mods.d/murdock.conf" "${destination}/"
printf '%s\n%s\n' "url: ${module_url}" "sha256: ${module_checksum}" > "${destination}/SOURCE"
cargo run --release --quiet -p haqor-cli -- db prepare murdock \
  --from "${work}/murdock.imp" --to "${destination}"
echo "Prepared Murdock's Peshitta into ${destination}"
