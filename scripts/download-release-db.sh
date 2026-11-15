#!/usr/bin/env bash
# Install the latest published haqor-core runtime database and sync the app.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ $# -ne 0 ]]; then
  echo "usage: $0" >&2
  exit 2
fi
for tool in gh jq sha256sum sqlite3; do
  command -v "$tool" >/dev/null || { echo "$tool is required" >&2; exit 1; }
done

repo=machshev/haqor-core
tag="$(gh release view --repo "$repo" --json tagName --jq .tagName)"
[[ -n "$tag" ]] || { echo "Could not find the latest release tag" >&2; exit 1; }

mkdir -p data
staging="$(mktemp -d "$root/data/.release-db.XXXXXXXX")"
trap 'rm -rf "$staging"' EXIT
gh release download "$tag" --repo "$repo" \
  --pattern 'haqor.db*' --dir "$staging"

for file in haqor.db haqor.db.sha256 haqor.db.manifest.json; do
  [[ -s "$staging/$file" ]] || { echo "Release $tag lacks $file" >&2; exit 1; }
done

(cd "$staging" && sha256sum --check --status haqor.db.sha256) || {
  echo "Release $tag database checksum failed" >&2
  exit 1
}

manifest="$staging/haqor.db.manifest.json"
database="$staging/haqor.db"
digest="$(sha256sum "$database")"
digest="${digest%% *}"
size="$(wc -c < "$database")"
jq -e --arg tag "$tag" --arg digest "$digest" --argjson size "$size" \
  '.schema_version == 1 and .core_ref == $tag and
   .blob_codec == "zstd" and .db_sha256 == $digest and
   .db_size == $size and (.db_built | type == "string" and length > 0)' \
  "$manifest" >/dev/null || { echo "Release $tag manifest does not match the database" >&2; exit 1; }

[[ "$(sqlite3 "$database" 'PRAGMA quick_check;')" == ok ]] || {
  echo "Release $tag database failed SQLite integrity check" >&2
  exit 1
}
built="$(sqlite3 "$database" "SELECT value FROM meta WHERE key = 'built';")"
codec="$(sqlite3 "$database" "SELECT value FROM meta WHERE key = 'blob_codec';")"
[[ "$built" == "$(jq -r .db_built "$manifest")" && "$codec" == zstd ]] || {
  echo "Release $tag database metadata does not match its manifest" >&2
  exit 1
}

mv -f "$database" data/haqor.db
echo "Installed $tag database (built $built) at $root/data/haqor.db"

app="$root/../haqor"
if [[ -f "$app/tool/sync-dbs.sh" ]]; then
  (cd "$app" && bash tool/sync-dbs.sh)
  echo "Synced the database into $app/assets/db/."
fi
