#!/usr/bin/env bash
set -euo pipefail

# Update the vendored Theographic Bible Metadata (CC BY-SA 4.0) in
# src_texts/Theographic: the events of the Bible, each with its year, how
# long it lasted, the verses telling it and the people and places taking
# part. Its chronology is a young-earth one, from the creation in 4004 BC
# (Floyd Nolen Jones, The Chronology of the Old Testament).
#
# `haqor db prepare theographic` keeps only the events and the names and
# verses of the people and places they name (events.jsonl, names.jsonl), and
# that is what is committed. This script downloads the CSV files at a pinned
# commit, verifies their checksums and prepares them; review the diff before
# committing.
readonly commit="cfb1c485d4da6fb63a69cb3b7f5b0752792f46bc"
readonly destination="${1:-src_texts/Theographic}"
readonly repository="https://github.com/robertrouse/theographic-bible-metadata"
readonly base_url="https://raw.githubusercontent.com/robertrouse/theographic-bible-metadata/${commit}"

readonly files=(Events.csv People.csv Places.csv)
readonly checksums=(
  "3325439a8d56d9a9f40895d26b119bfd82e5c21ceb07b93fd2e69eec30850a98"
  "44aa63e656077ed02a05747f581c56b5c9242a2d8cf7281344bc734085e0b130"
  "1b27e1a9a2bb93de9442de26b0c1cd11ca721348e06decd9b20acc0502759611"
)

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

mkdir -p "${work}/CSV"
for i in "${!files[@]}"; do
  curl --fail --silent --show-error --location \
    --output "${work}/CSV/${files[$i]}" "${base_url}/CSV/${files[$i]}"
  printf '%s  %s\n' "${checksums[$i]}" "${work}/CSV/${files[$i]}" | sha256sum --check --quiet -
done

mkdir -p "${destination}"
cargo run --release --quiet -p haqor-cli -- db prepare theographic \
  --from "${work}" --to "${destination}"
printf '%s\n%s\n%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
  "licence: CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/); prepared files are adapted from it and shared under the same licence" \
  "prepared by haqor db prepare theographic from CSV/Events.csv, CSV/People.csv and CSV/Places.csv" \
  > "${destination}/SOURCE"
echo "Prepared Theographic Bible Metadata ${commit} into ${destination}"
