# haqor-core

Rust libraries and tools providing Bible resources and the learning engine for
the Haqor app.

## Workspace

The repository root is a virtual Cargo workspace. Its packages are split by
responsibility:

- `crates/haqor-core`: app-facing Bible access, tutor, grammar, glosses, and text helpers
- `crates/haqor-morphology`: DB-free Hebrew morphology generation and parsing
- `crates/haqor-db-gen`: source-text parsing and generated-database pipelines
- `crates/haqor-admin`: loopback-only lexical-overlay web server
- `crates/haqor-cli`: the `haqor` command-line tool

Applications depend directly on `haqor-core`. Its public paths include
`haqor_core::bible`, `haqor_core::tutor`, and `haqor_core::morphology`.

## Data sources

The reader's primary Old Testament lemma and morphology data comes from the
[Open Scriptures Hebrew Bible](https://github.com/openscriptures/morphhb),
licensed under CC BY 4.0. Haqor's mechanically generated verb and noun analyses
remain in `hebrew.db` as reviewable alternatives and as a fallback where the
OSHB and UXLC token streams cannot be aligned safely.

## Commands

The CLI is the workspace's default member, so it remains available from the
workspace root:

```sh
cargo run -- db gen-hebrew --force
cargo run -- admin
```

### LAN progress sync

Run a personal server on the LAN that the app can reach:

```sh
cargo run -p haqor-sync-server --release -- \
  --bind 0.0.0.0:8788 \
  --progress "$HOME/.local/share/haqor/progress.db" \
  --token "choose-a-long-random-secret"
```

Then in the app open **Learn to read → Study pace → Progress sync**, enter the
machine's LAN address (for example `http://192.168.1.10:8788`) and the same
token. The app syncs when it launches and shortly after each answer. The
built-in service uses HTTP with a bearer token, so run it only on a trusted
LAN (or place it behind a VPN or HTTPS reverse proxy).

The admin server can also be run independently:

```sh
cargo run -p haqor-admin -- server --bind 127.0.0.1:8787
```

Build or test every package with:

```sh
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
```
