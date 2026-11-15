#!/usr/bin/env bash
# Rebuild every generation database before assembling the database used by Haqor.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

codec="${1:-zstd}"
if [[ $# -gt 1 || ( "$codec" != "zstd" && "$codec" != "none" ) ]]; then
  echo "usage: $0 [zstd|none]" >&2
  exit 2
fi

# Fresh checkouts leave these two Git submodules empty. The lexicon stage
# needs the XML, and the Hebrew stage needs morphhb's annotated WLC text.
if [[ ! -f src_texts/HebrewLexicon/HebrewStrong.xml || ! -d src_texts/morphhb/wlc ]]; then
  echo "Initializing HebrewLexicon and morphhb submodules..."
  git submodule update --init --depth 1 \
    src_texts/HebrewLexicon src_texts/morphhb
fi
if [[ ! -f src_texts/HebrewLexicon/HebrewStrong.xml || ! -d src_texts/morphhb/wlc ]]; then
  echo "Required source submodules are incomplete; check git submodule status." >&2
  exit 1
fi

cargo run --release -p haqor-cli -- db gen-bible
cargo run --release -p haqor-cli -- db gen-lexicon
cargo run --release -p haqor-cli -- db gen-hebrew --force
cargo run --release -p haqor-cli -- db gen-sedra
cargo run --release -p haqor-cli -- db gen-runtime --blob-codec "$codec"

echo "Built $root/data/haqor.db ($codec)."
app="$root/../haqor"
if [[ -f "$app/tool/sync-dbs.sh" ]]; then
  (cd "$app" && bash tool/sync-dbs.sh)
  echo "Synced the database into $app/assets/db/."
else
  echo "No sibling haqor checkout at $app; sync the database into the app when it is available."
fi
