#!/usr/bin/env bash
set -euo pipefail

# The unfoldingWord Literal Text (ULT) and the unfoldingWord Hebrew Bible
# (UHB) it is aligned to are fetched from their canonical repositories rather
# than redistributed here. Checking out pinned commits makes git
# verify every file against that commit's hashes, which keeps database
# generation reproducible. Only the Old Testament books are kept.
readonly ult_repository="https://git.door43.org/unfoldingWord/en_ult"
# Master rather than the v91 release, which leaves out the books still being
# checked (NUM, 1CH, 2CH, ECC, ISA, JER, EZK); their drafts are complete and
# fully aligned.
readonly ult_commit="d66da267710bc82d9936921b596558b139759a90" # master, 2026-10
readonly uhb_repository="https://git.door43.org/unfoldingWord/hbo_uhb"
readonly uhb_commit="74022f0fed012a3ef169886f595dd98e7b200543" # v3.0.0
readonly destination="${1:-src_texts/unfoldingWord}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

# fetch NAME REPOSITORY COMMIT
fetch() {
  local name="$1" repository="$2" commit="$3"
  git -c advice.detachedHead=false clone --quiet --filter=blob:none --no-checkout \
    "${repository}" "${work}/${name}"
  git -C "${work}/${name}" sparse-checkout set --no-cone \
    '/[0-3][0-9]-*.usfm' '/LICENSE.md' '/README.md' '/manifest.yaml'
  git -C "${work}/${name}" -c advice.detachedHead=false checkout --quiet "${commit}"
  mkdir -p "${destination}/${name}"
  # Books 01-39: the Old Testament (40 onwards, and A0 front matter, are not).
  find "${work}/${name}" -maxdepth 1 -name '[0-3][0-9]-*.usfm' \
    -exec cp {} "${destination}/${name}/" \;
  cp "${work}/${name}/LICENSE.md" "${work}/${name}/README.md" \
    "${work}/${name}/manifest.yaml" "${destination}/${name}/"
  printf '%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
    > "${destination}/${name}/SOURCE"
  local count
  count="$(find "${destination}/${name}" -name '*.usfm' | wc -l)"
  if [[ "${count}" -ne 39 ]]; then
    echo "expected 39 ${name} books, found ${count}" >&2
    exit 1
  fi
}

rm -rf "${destination}"
fetch en_ult "${ult_repository}" "${ult_commit}"
fetch hbo_uhb "${uhb_repository}" "${uhb_commit}"
echo "Fetched the ULT ${ult_commit} and UHB ${uhb_commit} into ${destination}"
