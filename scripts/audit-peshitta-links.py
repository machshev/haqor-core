#!/usr/bin/env python3
"""Inventory shared Peshitta spans and verify chapter review checkpoints.

This checks linkage and review records, not linguistic accuracy. Use the chapter
exporter's forms, morphology and gloss alternatives for the actual review.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import sqlite3


def spans(text):
    """Read the stored markup, rejecting unlinked lexical text and bad links."""
    result = []
    i = 0
    supplied = False
    while i < len(text):
        char = text[i]
        i += 1
        if char == "\\":
            if i == len(text) or text[i].isalnum():
                raise ValueError("invalid escape or unlinked lexical text")
            i += 1
        elif char in "{}":
            if (char == "{") == supplied:
                raise ValueError("unbalanced supplied markup")
            supplied = char == "{"
        elif char == "[":
            english = ""
            while i < len(text) and text[i] != "|":
                if text[i] == "\\":
                    i += 1
                    if i == len(text):
                        raise ValueError("unterminated escape")
                english += text[i]
                i += 1
            if i == len(text):
                raise ValueError("unterminated span")
            i += 1
            end = text.find("]", i)
            if end < 0:
                raise ValueError("unterminated links")
            raw = text[i:end].split(",")
            if not all(link.isascii() and link.isdecimal() for link in raw):
                raise ValueError("invalid or cross-verse link")
            if not any(c.isalnum() for c in english):
                raise ValueError("empty linked English")
            result.append((english, tuple(map(int, raw))))
            i = end + 1
        elif char.isalnum() or char in "]|":
            raise ValueError("unlinked lexical text or malformed markup")
    if supplied:
        raise ValueError("unclosed supplied markup")
    return result


def source_rows(path):
    rows = {}
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        book, cv, text = line.split("\t", 2)
        chapter, verse = map(int, cv.split(":"))
        key = (int(book), chapter, verse)
        if key in rows:
            raise ValueError(f"{path}:{number}: duplicate verse {key}")
        rows[key] = text
    return rows


def chapter_hash(rows, book, chapter):
    content = "".join(f"{b}\t{c}:{v}\t{text}\n"
                      for (b, c, v), text in sorted(rows.items())
                      if (b, c) == (book, chapter))
    return hashlib.sha256(content.encode()).hexdigest()


def audit(rows, db, ledger):
    expected = defaultdict(list)
    forms = {}
    for ref, position, form in db.execute(
        "SELECT n.ref, n.ord, s.vocalised FROM nt_word n "
        "JOIN syriac_word s USING(word_id) ORDER BY n.ref, n.ord"
    ):
        key = (ref >> 16, ref >> 8 & 255, ref & 255)
        expected[key].append(position)
        forms[(*key, position)] = form
    if rows.keys() != expected.keys():
        raise ValueError(f"verse inventory mismatch: missing {expected.keys() - rows.keys()}, "
                         f"extra {rows.keys() - expected.keys()}")
    chapters = defaultdict(list)
    for key, text in sorted(rows.items()):
        parsed = spans(text)
        links = [p for _, positions in parsed for p in positions]
        if set(links) != set(expected[key]):
            raise ValueError(f"{key}: missing or invalid source positions")
        chapter = (key[0], key[1])
        # Include chapters with no shared spans in the review inventory too.
        chapters[chapter]
        for english, positions in parsed:
            if len(positions) > 1:
                chapters[chapter].append({
                    "verse": key[2], "english": english, "positions": list(positions),
                    "source": [forms[(*key, p)] for p in positions],
                })
    report = []
    for (book, chapter), shared in sorted(chapters.items()):
        checkpoint = ledger.get("chapters", {}).get(f"{book}:{chapter}")
        reviewed = False
        if checkpoint:
            if checkpoint["sha256"] != chapter_hash(rows, book, chapter):
                raise ValueError(f"{book}:{chapter}: review hash is stale")
            exceptions = checkpoint["exceptions"]
            signatures = lambda entries: Counter(
                (entry["verse"], entry["english"], tuple(entry["positions"]))
                for entry in entries)
            if signatures(shared) != signatures(exceptions):
                raise ValueError(f"{book}:{chapter}: exception inventory mismatch")
            if any(not entry.get("reason", "").strip() for entry in exceptions):
                raise ValueError(f"{book}:{chapter}: exception missing linguistic reason")
            reviewed = checkpoint.get("independent_review") is True
        report.append({"book": book, "chapter": chapter, "reviewed": reviewed,
                       "shared_span_count": len(shared), "shared_spans": shared})
    return {"verses": len(rows), "source_words": sum(map(len, expected.values())),
            "shared_spans": sum(len(v) for v in chapters.values()),
            "reviewed_chapters": sum(c["reviewed"] for c in report),
            "pending_chapters": sum(not c["reviewed"] for c in report),
            "chapters": report}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("src_texts/Haqor-Peshitta/translation.tsv"))
    parser.add_argument("--db", type=Path, default=Path("data/haqor.db"))
    parser.add_argument("--ledger", type=Path, default=Path("src_texts/Haqor-Peshitta/alignment-review.json"))
    parser.add_argument("--out", type=Path)
    parser.add_argument("--require-reviewed", action="store_true",
                        help="fail if any chapter still awaits independent review")
    args = parser.parse_args()
    with sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True) as db:
        report = audit(source_rows(args.source), db, json.loads(args.ledger.read_text()))
    if args.out:
        args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "chapters"}))
    if args.require_reviewed and report["pending_chapters"]:
        raise SystemExit("alignment review is incomplete")


if __name__ == "__main__":
    main()
