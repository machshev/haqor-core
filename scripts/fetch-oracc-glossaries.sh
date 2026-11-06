#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Akkadian and Sumerian glossaries in src_texts/Oracc
# from the Open Richly Annotated Cuneiform Corpus: every project's
# gloss-akk.json and gloss-sux.json, merged into one entry per lemma (citation
# form, guide word and part of speech) with its meanings, commonest normalised
# forms, attestation count, periods, and each project's entry id for citing
# it. The Haqor lexicon searches them for a root's Akkadian and Sumerian
# cognates and loans (ADR 7).
#
# The glossary files state "This data is released under the CC0 license".
#
# oracc.museum.upenn.edu does not send its intermediate certificate, so the
# chain is completed with the intermediate its certificate names (InCommon RSA
# Server CA 2) and verified against the system's roots, as a browser does.
#
# Usage: scripts/fetch-oracc-glossaries.sh [DESTINATION]

readonly base="https://oracc.museum.upenn.edu"
readonly intermediate="http://crt.sectigo.com/InCommonRSAServerCA2.crt"
readonly destination="${1:-src_texts/Oracc}"

for tool in jq curl openssl unzip; do
  command -v "${tool}" >/dev/null || { echo "${tool} not found; run this inside 'nix develop'" >&2; exit 1; }
done

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

roots="${SSL_CERT_FILE:-${NIX_SSL_CERT_FILE:-/etc/ssl/certs/ca-certificates.crt}}"
curl -sS --fail -o "${work}/intermediate.der" "${intermediate}"
openssl x509 -inform DER -in "${work}/intermediate.der" -out "${work}/intermediate.pem"
openssl verify -CAfile "${roots}" "${work}/intermediate.pem" >/dev/null
cat "${roots}" "${work}/intermediate.pem" > "${work}/ca.pem"
get() { curl -sS --fail --retry 3 --cacert "${work}/ca.pem" "$@"; }

# The glossaries are published only inside each project's JSON archive (the
# glossary URLs answer with an empty body). A top-level project's glossary
# covers its subprojects, so the subprojects' archives are fetched only for a
# project whose own archive has none.
get "${base}/json/" | grep -oE '[a-z0-9-]+\.zip' | sed 's/\.zip$//' | sort -u > "${work}/archives"

mkdir -p "${work}/gloss"
# Fetch one archive, keep its glossaries; true when it had any.
glossaries_of() {
  local archive="$1"
  get -o "${work}/archive.zip" "${base}/json/${archive}.zip"
  local found=1
  for member in $(unzip -Z1 "${work}/archive.zip" | grep -E '(^|/)gloss-(akk|sux)\.json$'); do
    local lang="${member##*gloss-}"
    unzip -p "${work}/archive.zip" "${member}" > "${work}/gloss/${archive}.${lang}"
    found=0
  done
  rm -f "${work}/archive.zip"
  return "${found}"
}

while read -r project; do
  if ! glossaries_of "${project}"; then
    while read -r sub; do
      glossaries_of "${sub}" || true
    done < <(grep -E "^${project}-" "${work}/archives")
  fi
done < <(grep -v -- '-' "${work}/archives")
# Some archives carry an empty glossary for a language they have no words in.
for file in "${work}"/gloss/*.json; do
  if [[ ! -s "${file}" ]]; then
    rm -f "${file}"
    continue
  fi
  jq -e '.type == "glossary"' "${file}" >/dev/null || { echo "${file} is not a glossary" >&2; exit 1; }
done

# One line per project entry, reduced.
for file in "${work}"/gloss/*.json; do
  jq -c '
    .project as $project | .lang as $lang | .license as $license
    | .entries[]?
    | {
        lang: $lang, cf, gw, pos, project: $project, id, license: $license,
        icount: (.icount // "0" | tonumber),
        meanings: [.senses[]?.mng // empty] | unique,
        norms: [.senses[]?.norms[]? | {n, c: (.icount // "0" | tonumber)}],
        periods: [.periods[]?.p] | unique
      }' "${file}"
done > "${work}/entries.jsonl"

# Every file must carry the licence the header above quotes.
if jq -r '.license' "${work}/entries.jsonl" | sort -u | grep -vqx 'This data is released under the CC0 license'; then
  echo "a glossary states a different licence:" >&2
  jq -r '"\(.project) \(.license)"' "${work}/entries.jsonl" | sort -u | grep -v 'CC0' >&2
  exit 1
fi

mkdir -p "${destination}"
# Merge by language, citation form, guide word and part of speech.
jq -s -c '
  group_by([.lang, .cf, .gw, .pos])[]
  | {
      lang: .[0].lang, cf: .[0].cf, gw: .[0].gw, pos: .[0].pos,
      icount: (map(.icount) | add),
      meanings: (map(.meanings[]) | unique),
      norms: ([.[].norms[]] | group_by(.n) | map({n: .[0].n, c: (map(.c) | add)})
              | sort_by(-.c) | .[:8] | map(.n)),
      periods: (map(.periods[]) | unique),
      entries: (sort_by(-.icount) | map([.project, .id]))
    }
  | select(.cf != null and .cf != "")
' "${work}/entries.jsonl" | jq -s -c 'sort_by(.lang, .cf, .gw, .pos) | .[]' > "${destination}/glossary.jsonl"

{
  echo "Source: ${base}/<project>/gloss-{akk,sux}.json"
  echo "Fetched: $(date -u +%Y-%m-%d)"
  echo "Licence stated in every file: This data is released under the CC0 license"
  echo "Projects with a glossary:"
  ls "${work}/gloss" | sed 's/\.json$//' | sort | tr '\n' ' '
  echo
  echo "Entries:"
  jq -r '.lang' "${destination}/glossary.jsonl" | sort | uniq -c
} > "${destination}/SOURCE"

echo "Wrote $(wc -l < "${destination}/glossary.jsonl") lemmas to ${destination}/glossary.jsonl"
