# Contributing guide

Thank you for investing your time in contributing to m2m.

## Development environment

The development environment is managed by [Nix flakes](https://nixos.wiki/wiki/Flakes). Running `nix develop` (or `nix-shell` for non-flake users) spawns a shell with the right Rust toolchain and `cargo-deny`.

If you do not want to use Nix, install [rustup](https://rust-lang.github.io/rustup/index.html) and pull the toolchain pinned by `rust-version` in `Cargo.toml`:

```
rustup update
```

- `cargo` (>= `v1.87`)
- `rustc` (>= `v1.87`, edition 2024)

## Build

```
cargo build
```

You can disable default [features](https://doc.rust-lang.org/cargo/reference/features.html) with `--no-default-features` and enable individual features with `--features feat1,feat2`.

For example, a Maildir-only release build:

```
cargo build --no-default-features --features maildir --release
```

## Project layout

m2m is a thin CLI on top of the [Pimalaya](https://github.com/pimalaya) libraries. Most of the work happens in companion crates rather than in this repository:

- [io-email](https://github.com/pimalaya/io-email): cross-protocol email client (`EmailClientStd`, shared `Envelope` / `Mailbox` / `Flag` types).
- [io-maildir](https://github.com/pimalaya/io-maildir): Maildir / Maildir++ I/O-free coroutines and the std-blocking `MaildirClient`. Hosts the `pub` behaviour quirks (`dovecot_keywords`, `keywords_header`, `strip_headers`, `maildir_plus`, `maildirpp_inbox`, `fs_layout`) consumed by m2m.
- [io-m2dir](https://github.com/pimalaya/io-m2dir): m2dir I/O-free coroutines and the std-blocking `M2dirClient`.
- [pimalaya/cli](https://github.com/pimalaya/cli): cross-binary CLI helpers (clap args, build-time env, spinner, structured logger).

When debugging, prefer extending the upstream `io-*` library before adding logic to this repository. The CLI here is intentionally a thin orchestrator over `EmailClientStd`.

## Testing

```
cargo test --all-features
```

Manual smoke checks live in the README; the project does not yet ship an end-to-end test harness.

## Code style

```
cargo fmt
cargo clippy --all-features -- -D warnings
```

Run both before opening a pull request.
