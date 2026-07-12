# Manual lexicon overlays

Edit `lexicon_overrides.json` to correct imported lexical data without changing
Rust source. The file has two arrays:

- `lexicon_entries` changes the root and base gloss selected for a pointed
  surface when the imported BDB data is absent or chooses the wrong homograph.
- `word_glosses` changes the learner-facing gloss. `note` is optional teaching
  text and `is_name` optionally marks a proper name.

For example:

```json
{
  "surface": "כִּי",
  "root": "",
  "gloss": "that; because; for; when"
}
```

and:

```json
{
  "surface": "כִּי",
  "gloss": "for, because, that, when",
  "note": "Optional learner-facing explanation"
}
```

Keep each surface unique within its array. Then rebuild the generated databases:

```sh
cargo run -- db gen-lexicon
cargo run -- db gen-hebrew -n 0
```

`gen-lexicon` validates the JSON before writing the `lexicon_overrides` and
`word_glosses` tables into `data/lexicon.db`. Invalid or duplicate entries stop
generation with the relevant array index.
