#!/usr/bin/env bash
set -euo pipefail

# Update the vendored unfoldingWord Literal Text (ULT), and the unfoldingWord
# Hebrew Bible (UHB) and Greek New Testament (UGNT) it is aligned to, in
# src_texts/unfoldingWord.
#
# The build does not read the USFM: `haqor db prepare unfoldingword` reduces
# it to the UHB's and UGNT's words and the ULT's verses with their
# alignments (uhb.tsv, ugnt.tsv and ult.tsv), and that is what is committed, beside each
# text's licence, readme and manifest. This script checks out pinned commits
# (git verifies every file against the commit's hashes) and prepares them;
# review the diff before committing.
readonly ult_repository="https://git.door43.org/unfoldingWord/en_ult"
# Master rather than the v91 release, which leaves out the books still being
# checked (NUM, 1CH, 2CH, ECC, ISA, JER, EZK); their drafts are complete and
# fully aligned.
readonly ult_commit="d66da267710bc82d9936921b596558b139759a90" # master, 2026-10
readonly uhb_repository="https://git.door43.org/unfoldingWord/hbo_uhb"
readonly uhb_commit="74022f0fed012a3ef169886f595dd98e7b200543" # v3.0.0
readonly ugnt_repository="https://git.door43.org/unfoldingWord/el-x-koine_ugnt"
readonly ugnt_commit="fc95b2b8aad08bb65ab54628ab685413a1139e97" # v0.34
readonly destination="${1:-src_texts/unfoldingWord}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

# fetch NAME REPOSITORY COMMIT BOOKS
#
# BOOKS is how many books the text has: unfoldingWord numbers them 01-39 for
# the Old Testament and 41-67 for the New (A0 front matter is not a book).
fetch() {
  local name="$1" repository="$2" commit="$3" books="$4"
  git -c advice.detachedHead=false clone --quiet --filter=blob:none --no-checkout \
    "${repository}" "${work}/${name}"
  git -C "${work}/${name}" sparse-checkout set --no-cone \
    '/[0-6][0-9]-*.usfm' '/LICENSE.md' '/README.md' '/manifest.yaml'
  git -C "${work}/${name}" -c advice.detachedHead=false checkout --quiet "${commit}"
  mkdir -p "${work}/books/${name}"
  find "${work}/${name}" -maxdepth 1 -name '[0-6][0-9]-*.usfm' \
    -exec cp {} "${work}/books/${name}/" \;
  local count
  count="$(find "${work}/books/${name}" -name '*.usfm' | wc -l)"
  if [[ "${count}" -ne "${books}" ]]; then
    echo "expected ${books} ${name} books, found ${count}" >&2
    exit 1
  fi
  mkdir -p "${destination}/${name}"
  cp "${work}/${name}/LICENSE.md" "${work}/${name}/README.md" \
    "${work}/${name}/manifest.yaml" "${destination}/${name}/"
  printf '%s\n%s\n' "repository: ${repository}" "commit: ${commit}" \
    > "${destination}/${name}/SOURCE"
}

rm -rf "${destination}"
fetch en_ult "${ult_repository}" "${ult_commit}" 66
fetch hbo_uhb "${uhb_repository}" "${uhb_commit}" 39
fetch el-x-koine_ugnt "${ugnt_repository}" "${ugnt_commit}" 27
cargo run --release --quiet -p haqor-cli -- db prepare unfoldingword \
  --from "${work}/books" --to "${destination}"
echo "Prepared the ULT ${ult_commit}, UHB ${uhb_commit} and UGNT ${ugnt_commit} into ${destination}"
