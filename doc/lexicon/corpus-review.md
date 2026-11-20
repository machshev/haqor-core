# Corpus review

Run `haqor-review` from the core repository. Rust schedules work in `.haqor-review/progress.sqlite` and publishes accepted edits to version-controlled corpus files. The source editions are `ot` (Hebrew and Biblical Aramaic), `peshitta` and `greek` (Textus Receptus). Each occurrence is identified by edition, book, chapter, verse and the reader's zero-based word position.

## Start a pilot

```sh
cargo build --release -p haqor-review
target/release/haqor-review seed --source ot --book 1 --chapter 3 --verse 1
target/release/haqor-review seed --source ot --book 1 --chapter 27 --verse 45
target/release/haqor-review run --max-agents 4
target/release/haqor-review status
target/release/haqor-review export > genesis-review.jsonl
```

`seed` reads the local runtime database, including its current English. OT inventory also reads the OSHB lemma and morphology at each position from the matching `data/hebrew.db`. Use `--data DIR` for another build. Seed a chapter by omitting the verse, a book by omitting the chapter, or the full inventory with `seed` alone. Seeding starts zero agents. Repeating a selection preserves identical verse packets and reopens changed ones.

Accepted word and verse results are committed locally as they finish. Run `git log --oneline -- src_texts/Haqor-Review data/lexicon src_texts/Haqor-Names` to see the edits. The publisher stages only its affected files.

Use the global `--state PATH` on each command to select a different queue. Stop the harness with Ctrl-C. The next `run` reclaims interrupted jobs and retains completed results. A filesystem lock gives a queue one writer. On Linux child workers also receive a termination signal if their coordinator dies.

## Dispatch and review

`run` writes progress to stderr. Each start and finish shows active agents against the shared limit, completed source verses against the seeded total, the passage, and the zero-based word position or verse review. After 30 seconds without a completion it lists the active jobs and their elapsed times. The verse count includes completed reviews from previous runs.

With the default Codex backend, the harness reads the signed-in account's weekly percentage remaining at startup and every five minutes using `codex app-server`. This background check uses the `timeout` command. If the check cannot obtain a weekly window, it reports usage as unavailable and review continues.

`--max-agents` limits the combined number of word and verse workers. The default is four. Each word job sees its marked position, verse text, source analysis, candidate articles and examples of earlier analyses. The worker returns a dictionary form, language, homograph qualifier where needed, root, morphology, sense, contextual gloss and evidence. Semitic keys use Hebrew script. The harness normalizes keys to Unicode NFC. Imported lemma numbers can inform a decision.

A verse becomes ready when every word result has passed validation and been saved. It gets the next free slot, so other slots can continue word work on later verses. The reviewer sees the word results and existing English. The harness checks that the reviewed glosses cover every position exactly once and that the English spans link to every source position in that verse. A reviewer can send specific words back with reasons. Their verse waits again. Lexical and translation judgments remain the reviewers' responsibility.

## Worker backends

- By default each job starts a fresh read-only `codex exec` session. Use `--model MODEL` to select its model. The process can consult local sources when a word needs additional evidence. CLI instructions, tool definitions and follow-up reads contribute to billed input.
- For smaller prompts use a chat-completions endpoint. Rust sends a fresh message list for every job using `curl`. A word may request one candidate article, up to 48,000 bytes, before answering. The endpoint needs to support JSON-object responses. Configure `HAQOR_REVIEW_API_KEY` in the environment if it expects a bearer token. Authenticated requests use curl 8.3 or newer.
- Use `--worker PATH` for a custom executable. It receives one UTF-8 prompt on stdin and writes a single JSON object to stdout. Each prompt specifies the output fields and ends with a JSON packet after the line `Source packet:`. The process owns its stdout protocol and can write diagnostics to stderr.

For example, with a local server already serving your chosen model:

```sh
target/release/haqor-review run --max-agents 4 \
  --chat-endpoint http://localhost:8000/v1/chat/completions --model "$REVIEW_MODEL"
```

`status` shows counts by edition, job type and state, plus reported token usage. The SQLite `attempt` table records elapsed milliseconds, reported usage and errors. A worker transport failure stops new dispatch and lets active workers finish. Restart with `run` after fixing the worker. Invalid results are retried up to `--max-attempts` (default three). Use `retry` to reset failed jobs. A worker can mark a disputed occurrence unresolved and record its reason. Its verse waits. `retry --evidence '...'` reopens unresolved occurrences in that queue with the supplied evidence.

## Dictionary changes and outputs

Each edition has one tracked JSON file per passage under `src_texts/Haqor-Review/<edition>/<book>/<chapter>/<verse>.json`. Accepted word positions appear immediately with their surface, dictionary form, root, morphology, sense, gloss and lexical evidence. The verse reviewer adds source-linked English, final contextual glosses and named entities to the same file. Partial files can therefore supply reviewed words while the verse is still waiting.

The coordinator serializes source publication. It validates Semitic article edits and checks their base revision before writing `data/lexicon/`. Those articles remain drafts for human sign-off. An agent whose candidate or verse context changed while it was working receives a fresh packet. Published identities live in `src_texts/Haqor-Names/review.json` and personal-name articles can acquire additional bearers. Accepted content is read back from the tracked verse files. SQLite retains scheduling state, input snapshots, retry information, publication references and pending work.

`run` recovers interrupted publications before starting agents. Use `haqor-review publish` to recover pending source edits without starting any workers. `export` still emits one JSON record per completed verse. The runtime database is a build product. Full runtime generation imports the tracked sources; `haqor db gen-translation` refreshes reviewed word and translation data in an existing runtime database.

To correct a dictionary key, capture and reopen all matching assignments in one transaction. Each worker receives the previous assignment and the reason. Add `--retire` when the old spelling must disappear. Future results using that exact language, lemma and qualifier will be rejected.

```sh
target/release/haqor-review reassign --language he --lemma "$OLD_LEMMA" \
  --qualifier "$QUALIFIER" --reason "$CORRECTION" --retire
target/release/haqor-review run --max-agents 4
```

The queue contains the detailed work and staged articles. For a consistent backup of an active queue use SQLite's backup command.

```sh
sqlite3 .haqor-review/progress.sqlite ".backup '/tmp/haqor-review-backup.sqlite'"
```

## People and places

The verse reviewer identifies each explicit person or place at its source position. The packet includes existing occurrence tags, candidate records for matching names, and descriptions that distinguish namesakes. The reviewer selects a bearer by its stable record key and gives evidence from the verse. If the dataset lacks the bearer, the reviewer proposes a new person or place with a distinguishing description. An unresolved identity keeps the verse open. When resuming a queue created before identity review, the harness reopens completed verses and retains their word analyses.

Live verse publication updates the source overlay in `src_texts/Haqor-Names/review.json` and adds newly identified bearers to personal-name articles. Rebuilding with `haqor db gen-names` applies the overlay to the runtime database. `haqor db gen-greek` also honours the reviewed Greek assignments on subsequent builds. Rebuild the Haqor lexicon with `haqor db gen-haqor-lexicon` for updated name articles. Peshitta and Greek positions follow their respective source texts.
