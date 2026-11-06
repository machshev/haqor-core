#!/usr/bin/env bash
set -euo pipefail

# Refresh the checked-in Wiktionary entries in src_texts/Wiktionary: the
# Semitic languages, Egyptian, Coptic and Sumerian, with each entry's glosses,
# roots, etymology, and its links to other languages' words (cognate,
# inherited, borrowed, derived) and descendants — the cognate graph the Haqor
# lexicon's briefs search (ADR 7).
#
# kaikki.org publishes Wiktionary as extracted by wiktextract, but only the
# larger languages as their own files, so this streams the whole raw dump
# (~3 GB compressed) and keeps the languages below. Arabic keeps its lemmas
# only, not the pages that are just inflected forms. The dump is refreshed
# weekly, which is why the output is checked in: running this is a deliberate
# refresh whose diff is reviewed like any other source change.
#
# Usage: scripts/fetch-wiktionary-semitic.sh [DESTINATION]
# Set WIKTEXTRACT_RAW to an already downloaded raw-wiktextract-data.jsonl(.gz)
# to skip the download.

readonly url="https://kaikki.org/dictionary/raw-wiktextract-data.jsonl.gz"
readonly destination="${1:-src_texts/Wiktionary}"

# Wiktionary language codes. Every `sem-…` code is kept too (Proto-Semitic and
# its subgroups, Ammonite, Old South Arabian, …).
readonly langs='he hbo akk uga phn xpu arc oar syc jpa tmr sam smp myz xna xsa xqt inm xhd gez ar gdq sqt tig am egy cop sux xeb obm xdm afa-pro'

for tool in jq curl gzip; do
  if ! command -v "${tool}" >/dev/null; then
    echo "${tool} not found; run this inside 'nix develop'" >&2
    exit 1
  fi
done

mkdir -p "${destination}"
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

if [[ -n "${WIKTEXTRACT_RAW:-}" ]]; then
  stream() { if [[ "${WIKTEXTRACT_RAW}" == *.gz ]]; then gzip -dc "${WIKTEXTRACT_RAW}"; else cat "${WIKTEXTRACT_RAW}"; fi; }
  dump_date="local copy $(date -u -r "${WIKTEXTRACT_RAW}" +%Y-%m-%dT%H:%M:%SZ)"
else
  curl -sI --fail "${url}" > "${work}/headers"
  dump_date="$(awk -F': ' 'tolower($1) == "last-modified" { print $2 }' "${work}/headers" | tr -d '\r' | tail -n 1)"
  stream() { curl -L --fail --retry 3 --silent --show-error "${url}" | gzip -dc; }
fi

# A cheap text prefilter on the top-level language code, then the exact
# selection and reduction in jq.
pattern="\"lang_code\": ?\"($(tr ' ' '|' <<< "${langs}")|sem-[a-z-]+)\""
stream | grep -E "${pattern}" | jq -c --arg langs "${langs}" '
  ($langs | split(" ")) as $keep
  | select(.lang_code as $l | ($keep | index($l)) or ($l | startswith("sem-")))
  # Arabic: lemmas only, not pages whose every sense is an inflected form
  # (marked form-of, or in the raw dump often just left without a gloss).
  | select(.lang_code != "ar" or ([.senses[]? | select((.form_of // []) == []
      and ((.tags // []) | index("form-of") | not)
      and ((.glosses // []) | map(select(. != "")) | length > 0))] | length > 0))
  | {
      word,
      lang: .lang_code,
      pos,
      etym: .etymology_number,
      roman: ([.forms[]? | select((.tags // []) | index("romanization")) | .form] | first),
      # Categories are objects in the per-language files and plain strings in
      # the raw dump, on the entry and on each sense.
      roots: ([(.categories[]?, .senses[]?.categories[]?)
               | if type == "object" then .name else . end
               | strings
               | select(test(" terms belonging to the root "))
               | sub("^.* terms belonging to the root "; "")] | unique),
      glosses: ([.senses[]? | (.glosses // .raw_glosses // []) | join("; ")
                 | select(. != "")] | .[:12]),
      etymology: .etymology_text,
      links: ([.etymology_templates[]?
               | select(.name | test("^(cog|ncog|inh|inh\\+|bor|bor\\+|der|der\\+|lbor|slbor|uder|obor|calque|root)$"))
               | [.name, .args["1"], .args["2"], .args["3"], .args.tr]
               | if .[0] == "cog" or .[0] == "ncog" then [.[0], .[1], .[2], .[4]]
                 elif .[0] == "root" then [.[0], .[2], .[3], null]
                 else [.[0], .[2], .[3], .[4]] end]),
      descendants: ([.descendants[]? | .. | objects
                     | select(.lang_code? and .word?) | [.lang_code, .word, .roman]] | unique)
    }
  | with_entries(select(.value != null and .value != [] and .value != ""))
' > "${work}/entries.jsonl" 2> "${work}/errors"

# jq skips a record it cannot reduce and carries on; a skipped record is a
# lost entry, so any error fails the refresh.
if [[ -s "${work}/errors" ]]; then
  head -n 5 "${work}/errors" >&2
  echo "jq could not reduce $(wc -l < "${work}/errors") records" >&2
  exit 1
fi

# Stable order, so a refresh diffs cleanly.
jq -s -c 'sort_by(.lang, .word, ((.etym // "0") | tonumber? // 0), .pos) | .[]' "${work}/entries.jsonl" \
  > "${destination}/semitic.jsonl"

{
  echo "Source: ${url}"
  echo "Dump: ${dump_date}"
  echo "Languages: ${langs} sem-*"
  echo "Entries:"
  jq -r '.lang' "${destination}/semitic.jsonl" | sort | uniq -c | sort -rn
} > "${destination}/SOURCE"

echo "Wrote $(wc -l < "${destination}/semitic.jsonl") entries to ${destination}/semitic.jsonl"
