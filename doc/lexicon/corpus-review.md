# Corpus review

Run `haqor-review` from the core repository. Rust schedules the work and stores the queue in `.haqor-review/progress.sqlite`. The source editions are `ot` (Hebrew and Biblical Aramaic), `peshitta` and `greek` (Textus Receptus). Each occurrence is identified by edition, book, chapter, verse and the reader's zero-based word position.

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

Use the global `--state PATH` on each command to select a different queue. Stop the harness with Ctrl-C. The next `run` reclaims interrupted jobs and retains completed results. A filesystem lock gives a queue one writer. On Linux child workers also receive a termination signal if their coordinator dies.

## Dispatch and review

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

Workers can cite a matching current article, reuse a previous lexical decision, or propose an evidenced draft. Semitic article changes are staged as complete Haqor JSON. Greek lexical drafts are stored with their occurrences. `export` emits one JSON line per completed source verse, including word assignments, lexical decisions, final contextual glosses and linked English. Treat this export as review data. The Flutter reader consumes the generated runtime database.

```sh
target/release/haqor-review apply-articles
target/release/haqor-review status
```

`apply-articles` publishes staged Semitic articles for completed verses into `data/lexicon` using Haqor's existing validator. They remain drafts for sign-off. An article changed since its proposal sends the affected occurrence back for reconciliation. Applying drafts and starting a run both check tracked article revisions and reopen dependent words and verse reviews. Commit verified changes in the usual way.

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
