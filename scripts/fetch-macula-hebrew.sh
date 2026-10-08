#!/usr/bin/env bash
set -euo pipefail

# Update the vendored MACULA Hebrew syntax trees in src_texts/MACULA-Hebrew.
#
# The build does not read MACULA's 400 MB of XML: `haqor db prepare
# macula-hebrew` reduces it to the trees alone (trees.tsv, about 13 MB), and
# that is what is committed. This script downloads a pinned commit (checking it
# out makes git verify every file against the commit's hashes) and prepares it;
# review the diff before committing.
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

# The chapter files only: `macula-hebrew-lowfat.xml` merely XIncludes them.
count="$(find "${work}/macula-hebrew/WLC/lowfat" -name '[0-9][0-9]-*-lowfat.xml' | wc -l)"
if [[ "${count}" -ne 929 ]]; then
  echo "expected 929 chapter files, found ${count}" >&2
  exit 1
fi

rm -rf "${destination}"
mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db prepare macula-hebrew \
  --from "${work}/macula-hebrew/WLC/lowfat" --to "${destination}"
cp "${work}/macula-hebrew/LICENSE.md" "${work}/macula-hebrew/README.md" "${destination}/"
printf '%s\n%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
  "prepared: WLC/lowfat/*-lowfat.xml reduced to trees.tsv by haqor db prepare macula-hebrew" \
  > "${destination}/SOURCE"
echo "Prepared MACULA Hebrew ${commit} into ${destination}"
