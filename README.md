# haqor-core

Rust libraries and tools providing Bible resources and the learning engine for
the Haqor app.

## Workspace

The root `haqor-core` package is the compatibility facade used by downstream
applications. Its public runtime module paths remain stable, including
`haqor_core::bible`, `haqor_core::tutor`, and `haqor_core::morphology`.

The implementation is split by responsibility:

- `crates/haqor-runtime`: Bible access, tutor, grammar, glosses, and text helpers
- `crates/haqor-morphology`: DB-free Hebrew morphology generation and parsing
- `crates/haqor-data`: source-text parsing and generated-database pipelines
- `crates/haqor-admin`: loopback-only lexical-overlay web server

Tooling is enabled by the root package's default `tools` feature. Consumers
that only need the app runtime can use `default-features = false`, as the Haqor
Flutter bridge does, avoiding generator, CLI, and admin dependencies.

## Commands

The existing CLI remains available from the workspace root:

```sh
cargo run -- db gen-hebrew --force
cargo run -- admin
```

The admin server can also be run independently:

```sh
cargo run -p haqor-admin -- --bind 127.0.0.1:8787
```

Build or test every package with:

```sh
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
```
