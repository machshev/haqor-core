#!/usr/bin/env python3
"""Export Murdock and exact Peshitta word context for human alignment review."""
import argparse
import json
import sqlite3
from pathlib import Path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", type=Path, default=Path("data/haqor.db"))
    ap.add_argument("--murdock", type=Path, default=Path("src_texts/Murdock/murdock.tsv"))
    ap.add_argument("--translation", type=Path,
                    default=Path("src_texts/Haqor-Peshitta/translation.tsv"))
    ap.add_argument("--book", type=int, default=40, help="Haqor book number (Matthew=40)")
    ap.add_argument("--first-chapter", type=int, default=1)
    ap.add_argument("--last-chapter", type=int)
    ap.add_argument("--out", type=Path, default=Path("/tmp/haqor-nt-review.jsonl"))
    args = ap.parse_args()
    last = args.last_chapter or args.first_chapter
    translations = {}
    for line in args.translation.read_text().splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        book, cv, text = line.split("\t", 2)
        chapter, verse = map(int, cv.split(":"))
        if int(book) == args.book and args.first_chapter <= chapter <= last:
            translations[(chapter, verse)] = text
    murdock = {}
    with args.murdock.open(encoding="utf-8") as f:
        for line in f:
            book, cv, text = line.rstrip("\n").split("\t", 2)
            if int(book) != args.book:
                continue
            chapter, verse = map(int, cv.split(":", 1))
            if args.first_chapter <= chapter <= last:
                murdock[(chapter, verse)] = text
    db = sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    sql = """
      SELECT (n.ref >> 16) AS book, ((n.ref >> 8) & 255) AS chapter,
             (n.ref & 255) AS verse, n.ord AS position,
             sw.word AS surface, sw.vocalised, sw.lexeme_id,
             sl.lexeme, sg.before AS gloss_before, sg.meaning, sg.after AS gloss_after,
             sw.gender, sw.person, sw.number, sw.state, sw.tense, sw.form,
             sw.suffix_person, sw.suffix_gender, sw.suffix_number
      FROM nt_word n
      JOIN syriac_word sw ON sw.word_id = n.word_id
      LEFT JOIN syriac_lexeme sl ON sl.lexeme_id = sw.lexeme_id
      LEFT JOIN syriac_gloss sg ON sg.lexeme_id = sw.lexeme_id
      WHERE (n.ref >> 16) = ? AND ((n.ref >> 8) & 255) BETWEEN ? AND ?
      ORDER BY n.ref, n.ord, sg.gloss_id
    """
    rows = db.execute(sql, (args.book, args.first_chapter, last))
    verses = {}
    for row in rows:
        key = (row["chapter"], row["verse"])
        word = {k: row[k] for k in row.keys() if k not in ("book", "chapter", "verse")}
        # Keep all SEDRA gloss alternatives, but emit one source-word record.
        if key not in verses:
            verses[key] = {"book": args.book, "chapter": key[0], "verse": key[1],
                            "murdock": murdock.get(key, ""),
                            "translation": translations.get(key, ""), "words": []}
        words = verses[key]["words"]
        if words and words[-1]["position"] == word["position"]:
            words[-1]["glosses"].append({"before": word.pop("gloss_before"),
                                         "meaning": word.pop("meaning"),
                                         "after": word.pop("gloss_after")})
        else:
            gloss = {"before": word.pop("gloss_before"), "meaning": word.pop("meaning"),
                     "after": word.pop("gloss_after")}
            word["glosses"] = [gloss]
            words.append(word)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as f:
        for key in sorted(verses):
            f.write(json.dumps(verses[key], ensure_ascii=False, separators=(",", ":")) + "\n")
    print(f"wrote {len(verses)} verses to {args.out}")


if __name__ == "__main__":
    main()
