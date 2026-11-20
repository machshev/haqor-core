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

`--max-agents` limits the combined number of word and verse workers. The default is four. Each word job sees its marked position, verse text, source analysis, candidate articles and examples of earlier analyses. The worker returns a dictionary form, language, homograph qualifier where needed, root, morphology, sense, contextual gloss and evidence. The morphology is machine readable. For Hebrew, Aramaic and Syriac, `morph` is an OSHB morphology code (`H` Hebrew, `A` Aramaic and Syriac; for example `HC/Vpw3ms`) and `segments` is the word split into the same parts (`וַ/יְבָרֶךְ`). For Greek, `morph` is a Robinson code. The harness checks the code against the OSHB grammar and checks that the segments spell the word. A prose `morphology` explains the reading. Its entry names a lexicon article and `word`, the index of the lexeme in that article's `words`. It also names `sense`, the id of the sense the occurrence bears, from the lexeme's senses as the candidate lists them. Every lexicon sense has an id. The harness checks that this lexeme has the returned dictionary form, language and root. A draft names the index in its draft article. A published draft may change a lexeme's content or add lexemes at the end. It cannot move or remove an existing lexeme, or drop an existing sense id, so issued indices and ids stay valid. Semitic keys use Hebrew script. The harness normalizes keys to Unicode NFC. Imported lemma numbers can inform a decision.

A word need not reach an agent at all. Some words have the same surface and source tagging (for the OT, the OSHB lemma with its Strong's number, and its morphology) as words already reviewed. If at least two such reviews agree on dictionary form, root, morphology, published article, lexeme and sense, the harness reuses that analysis and the word passes the usual checks. Older reviews without a sense id do not count toward the agreement. The verse reviewer sees these words marked `carried` and confirms they fit the verse, or sends them back. A word with feedback or an earlier error always goes to an agent.

A verse becomes ready when every word result has passed validation and been saved. It gets the next free slot, so other slots can continue word work on later verses. The reviewer sees the word results and existing English. The harness checks that the reviewed glosses cover every position exactly once and that the English spans link to every source position in that verse. A reviewer can send specific words back with reasons. Their verse waits again. Lexical and translation judgments remain the reviewers' responsibility.

## Worker backends

- By default each job starts a fresh read-only `codex exec` session. Use `--model MODEL` to select its model. Use `--verse-model` to give verse reviews a different model, and `--effort` and `--verse-effort` to set Codex reasoning effort for word and verse jobs. Unset options take the values in `~/.codex/config.toml`. The process can consult local sources when a word needs additional evidence. CLI instructions, tool definitions and follow-up reads contribute to billed input.
- `--escalate-model` and `--escalate-effort` route escalated jobs differently from first attempts. An escalated job is a retry after an error, a word a verse reviewer sent back, or a reopened occurrence. With a chat endpoint as well, first attempts use the endpoint and escalated jobs use Codex with these settings.
- `--draft-model` and `--draft-effort` give article drafting its own route. When either is set, other word workers may not draft. A word that no article covers returns `{"needs_draft": reason}` and is requeued for the drafting route without spending an attempt. A routine worker that drafts anyway has its draft turned into such a request. The drafting worker sees the reason as `draft_requested`.
- For smaller prompts use a chat-completions endpoint. Rust sends a fresh message list for every job using `curl`. A word may request one candidate article, up to 48,000 bytes, before answering. The endpoint needs to support JSON-object responses. Configure `HAQOR_REVIEW_API_KEY` in the environment if it expects a bearer token. Authenticated requests use curl 8.3 or newer.
- Use `--worker PATH` for a custom executable. It receives one UTF-8 prompt on stdin and writes a single JSON object to stdout. Each prompt specifies the output fields and ends with a JSON packet after the line `Source packet:`. The process owns its stdout protocol and can write diagnostics to stderr.

For example, with a local server already serving your chosen model:

```sh
target/release/haqor-review run --max-agents 4 \
  --chat-endpoint http://localhost:8000/v1/chat/completions --model "$REVIEW_MODEL"
```

`status` shows counts by edition, job type and state, plus reported token usage. Each attempt records its route: job kind, purpose (first, escalated, draft or carried), backend, model and effort. `status` lists, per route, the attempts, the failed attempts, the accepted results later redone (sent back, reopened or handed to drafting), and the average input tokens. Use it to compare routes on a pilot chapter before choosing defaults. One starting point keeps routine words on a fast model and gives judgement work the strongest one:

```sh
target/release/haqor-review run --max-agents 4 --model gpt-6.2-sol \
  --verse-model gpt-6.2-astra --verse-effort high \
  --escalate-model gpt-6.2-astra --escalate-effort high \
  --draft-model gpt-6.2-astra --draft-effort xhigh
``` The SQLite `attempt` table records elapsed milliseconds, reported usage and errors. A worker transport failure stops new dispatch and lets active workers finish. Restart with `run` after fixing the worker. Invalid results are retried up to `--max-attempts` (default three). Use `retry` to reset failed jobs. A worker can mark a disputed occurrence unresolved and record its reason. Its verse waits. `retry --evidence '...'` reopens unresolved occurrences in that queue with the supplied evidence.

## Dictionary changes and outputs

Each edition has one tracked JSON file per passage under `src_texts/Haqor-Review/<edition>/<book>/<chapter>/<verse>.json`. Accepted word positions appear immediately with their surface, dictionary form, root, morphology, sense, gloss and lexical evidence. The verse reviewer adds source-linked English, final contextual glosses and named entities to the same file. Partial files can therefore supply reviewed words while the verse is still waiting.

Workers never name articles. The harness derives every id from the article's content: a root's entry from the root, a noun with no verbal root from its lemma (filed under `nouns/`), and a personal name from the name. If a draft turns a noun into its root's entry, by adding a verb or a second lexeme, the root entry replaces the noun. The noun's file is removed and its readers are repointed in the same commit. The coordinator serializes source publication. It validates Semitic article edits and checks their base revision before writing `data/lexicon/`. Those articles remain drafts for human sign-off. Greek drafts share files under `data/lexicon/greek/` keyed by dictionary form and homograph qualifier. An agent whose candidate or verse context changed while it was working receives a fresh packet. Published identities live in `src_texts/Haqor-Names/review.json` and personal-name articles can acquire additional bearers. Accepted content is read back from the tracked verse files. SQLite retains scheduling state, input fingerprints, retry information, publication references and pending work.

`run` recovers interrupted publications before starting agents. Use `haqor-review publish` to recover pending source edits without starting any workers. `export` still emits one JSON record per completed verse. The runtime database is a build product. The build merges each reviewed analysis into the word tables, so the runtime holds one answer per word. An OT word points at a reviewed `word_info` row with its root, gloss in context, morphology cell, lexicon article and lexeme. A Peshitta word's `nt_word` row carries the same. A Greek word's lemma, English and grammar ids point at the reviewed values. Each merged row keeps a reference to the generated analysis it replaced. Packets read through that reference, so a review never becomes its own source evidence. `haqor db gen-translation` reapplies reviewed words and translations to an existing runtime database. Run it again after `gen-greek`, which rebuilds the Greek words.

Occurrences reviewed before lexeme indices were required can be linked with `haqor-review link-lexemes`. It writes the index where exactly one lexeme matches and reopens the remaining occurrences with the article's lexemes listed. `haqor-review link-senses` records the sense id where the lexeme has one sense or the prose sense matches one gloss. Other occurrences keep their prose sense until they are next reviewed. `haqor-review fill-morph` gives the same treatment to morphology. An OT occurrence whose prose agrees with the OSHB reading at that position takes the OSHB code and segments. The others, and every Peshitta occurrence, are reopened. Lexicon builds warn when a lexeme index no longer names the reviewed word.

`haqor-review audit` lists words whose reviews of one surface and source tagging disagree on the lexical reading, and OT readings that depart from the OSHB. It does not report sense differences, because one word can bear different senses in different verses. Add `--reopen` to send minority readings back with the majority reading as feedback.

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
