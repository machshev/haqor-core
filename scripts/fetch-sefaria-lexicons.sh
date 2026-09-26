#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Klein and Jastrow entries in src_texts/Sefaria from
# Sefaria's nightly MongoDB dump.
#
# Sefaria publishes no per-lexicon download, only the whole database (~2.5 GB).
# The archive is streamed and only the `lexicon_entry` collection is written to
# disk, so the refresh needs a few hundred MB of scratch space, not gigabytes.
# The dump changes nightly, which is why its output is checked in: the build
# reads src_texts/Sefaria, and running this script is a deliberate refresh
# whose diff is reviewed like any other source change.
#
# Needs `bsondump` (MongoDB Database Tools), which the dev shell provides.
#
# Usage: scripts/fetch-sefaria-lexicons.sh [DESTINATION]
# Set SEFARIA_LEXICON_BSON to an already extracted lexicon_entry.bson to skip
# the download.

readonly url="https://storage.googleapis.com/sefaria-mongo-backup/dump_small.tar.gz"
readonly destination="${1:-src_texts/Sefaria}"

if ! command -v bsondump >/dev/null; then
  echo "bsondump not found; run this inside 'nix develop'" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

if [[ -n "${SEFARIA_LEXICON_BSON:-}" ]]; then
  bson="${SEFARIA_LEXICON_BSON}"
  dump_date="local copy $(date -u -r "${bson}" +%Y-%m-%dT%H:%M:%SZ)"
else
  curl -L --fail --retry 3 --dump-header "${work}/headers" "${url}" |
    tar xz -C "${work}" --wildcards '*/lexicon_entry.bson'
  bson="$(find "${work}" -name lexicon_entry.bson -print -quit)"
  dump_date="$(awk -F': ' 'tolower($1) == "last-modified" { print $2 }' "${work}/headers" |
    tr -d '\r' | tail -n 1)"
fi

bsondump --quiet --outFile "${work}/lexicon_entry.jsonl" "${bson}"
bson_sha256="$(sha256sum "${bson}" | cut -d' ' -f1)"

cargo run --release -p haqor-cli -- db import-sefaria \
  --input "${work}/lexicon_entry.jsonl" \
  --output "${destination}" | tee "${work}/summary"

{
  echo "url: ${url}"
  echo "dump: ${dump_date}"
  echo "lexicon_entry.bson sha256: ${bson_sha256}"
  cat "${work}/summary"
} > "${destination}/SOURCE"

echo "Refreshed ${destination} from the Sefaria dump of ${dump_date}"
