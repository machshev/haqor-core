# Manual lexicon overlays

Edit `lexicon_overrides.json` to correct imported lexical data without changing
Rust source. The file has three arrays:

- `lexicon_entries` changes the root and base gloss selected for a pointed
  surface when the imported BDB data is absent or chooses the wrong homograph.
- `word_glosses` changes the learner-facing gloss. `note` is optional teaching
  text and `is_name` optionally marks a proper name.
- `primary_analyses` pins one of `hebrew.db`'s verb or noun analyses as the
  primary reading for a surface. Use the browser editor to avoid transcribing
  its morphology fields by hand.

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

## Browser editor

Run the loopback-only admin server and open the printed address:

```sh
cargo run -- admin
```

The default is `http://127.0.0.1:8787`. Saving validates the full document and
atomically replaces the overlay file. The server deliberately refuses a
non-loopback bind because it has no authentication.

The **Imported glosses** tab browses the BDB and Strong's glosses in
`data/lexicon.db`. Change the proposed gloss and choose **Create overlay** to
add or update a `lexicon_entries` row; the imported database row is never
edited directly. Use `--lexicon PATH` when browsing a database elsewhere.

The **Ambiguous analyses** tab shows the 500 highest-frequency ambiguous
surfaces from `data/hebrew.db`. Choosing a non-default reading creates a
regeneration-safe `primary_analyses` override; **Automatic selection** removes
it. Verb and noun candidates are both shown. Rebuild `lexicon.db` after saving.
The alternatives remain in `hebrew.db`; only the primary reading used by the
tutor changes. Use `--hebrew PATH` to review another generated database.
