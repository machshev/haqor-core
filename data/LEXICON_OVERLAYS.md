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
  "gloss": "for"
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

## Mobile tutor corrections

Tutor admin mode stores quick, mobile-made gloss corrections in the app's
`progress.db`. They are included in ordinary LAN progress sync, so review them
on the machine that runs the sync server and merge them into the checked-in
overlay with:

```sh
haqor-admin pull --progress data/sync-progress.db --overlay data/lexicon_overrides.json
```

When the sync-server database is not mounted locally, pull the authenticated
snapshot over the LAN instead:

```sh
haqor-admin pull --server http://192.168.1.10:8788 --token 'your sync token'
```

With no source options, `haqor-admin pull` reads the saved server URL and token
from `$XDG_DATA_HOME/org.haqor/shared_preferences.json` (or
`$HOME/.local/share/org.haqor/shared_preferences.json` when `XDG_DATA_HOME` is
unset). It temporarily falls back to the old Flutter-template application ID
so an upgrade does not lose existing sync settings. Use `--progress` to force a
local database or pass `--server` / `--token` to override either saved value.

The command updates `word_glosses` atomically, preserving an existing proper-name
marker. Regenerate the lexical databases afterwards as usual.

## Mobile issue and idea log

Admin mode also exposes a bug/idea flag in word information and on every tutor
card. Each entry includes the visible word or complete tutor-card payload,
platform details, and the reporter's note. Reports are stored in `progress.db`
and travel through the same ordinary LAN snapshot sync as tutor gloss
corrections.

Download the canonical log to a local, gitignored JSON file with either a
direct sync-server database path:

```sh
haqor-admin pull-issues --progress data/sync-progress.db
```

or the authenticated LAN endpoint:

```sh
haqor-admin pull-issues --server http://192.168.1.10:8788 --token 'your sync token'
```

With no source options, `pull-issues` uses the same saved app sync settings as
`pull`. The default output is `data/issue_reports.json`; use `--output PATH` to
write elsewhere. Each download atomically replaces the file with the complete
canonical report set.

The **Ambiguous analyses** tab shows the 500 highest-frequency ambiguous
surfaces from `data/hebrew.db`. Choosing a non-default reading creates a
regeneration-safe `primary_analyses` override; **Automatic selection** removes
it. Verb and noun candidates are both shown. Rebuild `lexicon.db` after saving.
The alternatives remain in `hebrew.db`; only the primary reading used by the
tutor changes. Use `--hebrew PATH` to review another generated database.
