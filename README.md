# m2m [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya)

CLI to convert filesystem-based mailboxes, written in Rust.

> [!CAUTION]
> m2m is in active development and shipped as `v0.x.x`. Expect breaking changes between releases until stabilisation.

## Table of contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Behaviour knobs](#behaviour-knobs)
- [Interfaces](#interfaces)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- Read from and write to **Maildir** <sup>[specs](https://cr.yp.to/proto/maildir.html)</sup>, **Maildir++** <sup>[specs](https://www.courier-mta.org/imap/README.maildirquota.html)</sup> and **m2dir** <sup>[specs](https://man.sr.ht/~bitfehler/m2dir/)</sup> filesystem stores.
- **Single-shot conversion** by chained subcommands: `m2m <SOURCE-CMD> <PATH> [src-flags] <TARGET-CMD> <PATH> [tgt-flags]`. The destination store is created on first run.
- **Parallel copy loop** sized to `--workers` (default = available parallelism, capped at 16).
- **Idempotent re-runs** via `--skip-existing`, which dedup-matches by `Message-ID` then by source id.
- **Dry-run mode** (`--dry-run`) lists what would be touched without creating or modifying any file.
- **Inbox detection** via `--inbox <NAME>` (default `INBOX`): case-insensitively identifies the source inbox folder and renames it to the canonical alias on the target. Maildir++ root surfaces under the same alias.
- **Custom keyword round-trip** between Maildir-family stores via two configurable side channels: the per-folder `dovecot-keywords` slot file (Dovecot / mbsync convention) and a body header (`X-Keywords` or `X-Label`).
- **Header stripping** on read (`--strip-headers`) to drop legacy markers (`X-Mozilla-Status`, `X-Status`, ...).
- **Maildir++ layout awareness**: the `maildir++` subcommand reads / writes dotted folder siblings (`.Work.Foo/`) and reports them as logical `Work/Foo` mailboxes.
- **Dovecot fs-layout** via `--fs-layout` on the `maildir` subcommand: nested subfolders (`Work/Foo/`) instead of flat dotted siblings.

> [!TIP]
> m2m is written in [Rust](https://www.rust-lang.org/) and uses [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to gate backend support. The default feature set is declared in [Cargo.toml](./Cargo.toml).

## Installation

### Cargo

```
cargo install --locked --git https://github.com/pimalaya/m2m.git
```

### Nix

If you have the [Flakes](https://nixos.wiki/wiki/Flakes) feature enabled:

```
nix profile install github:pimalaya/m2m
```

Or run without installing:

```
nix run github:pimalaya/m2m
```

### Sources

```
git clone https://github.com/pimalaya/m2m
cd m2m
nix run
```

## Usage

The grammar is a chain of two subcommands, one per side, each carrying its own positional `<PATH>` and per-format flags:

```
m2m [SHARED-FLAGS] <SOURCE-CMD> <PATH> [SOURCE-FLAGS] <TARGET-CMD> <PATH> [TARGET-FLAGS]
```

Backends accepted as both source and target: `maildir`, `maildir++`, `m2dir`.

Examples:

```
m2m maildir ~/Mail m2dir ~/.Mail/example
m2m --workers 8 --skip-existing maildir ~/Mail --dovecot-keywords --keywords-header x-keywords m2dir ~/.Mail
m2m m2dir ~/.Mail maildir++ ~/Maildir
m2m --dry-run maildir ~/Mail m2dir /mnt/backup
```

### Shared flags

| Flag | Purpose |
| --- | --- |
| `-w`, `--workers <N>` | Worker threads (default = available parallelism, capped at 16). |
| `--inbox <NAME>` | Canonical inbox folder name (default `INBOX`); detected case-insensitively on the source, used as-is on the target. |
| `--include-mailbox <NAME>` | Process only these mailboxes (repeatable, case-insensitive). |
| `--exclude-mailbox <NAME>` | Skip these mailboxes (repeatable, case-insensitive). |
| `--skip-existing` | Skip messages already present on the target. |
| `--dry-run` | List the conversion plan without writing anything. |
| `--json` | Machine output instead of TTY spinner. |
| `--log-level <off,error,warn,info,debug,trace>` | Filter log output. |

## Behaviour knobs

`maildir` accepts four optional flags on either side, `maildir++` accepts three (no `--fs-layout`), `m2dir` accepts none.

| Flag | What it does |
| --- | --- |
| `--dovecot-keywords` | Reads / writes the per-folder `dovecot-keywords` sidecar (Dovecot / mbsync convention). Custom keywords in filenames (`a..z`) round-trip through the slot table. |
| `--keywords-header <x-keywords\|x-label>` | Mirrors custom keywords into the chosen body header. `x-keywords` follows the OfflineIMAP / mbsync convention (comma-separated); `x-label` follows the mutt / notmuch convention (space-separated). |
| `--strip-headers <CSV>` | Drops these header names from each message on read; useful to clean Mozilla / Pegasus markers (`X-Mozilla-Status`, `X-Status`, ...). |
| `--fs-layout` | Dovecot fs layout: subfolders are nested filesystem directories (`Work/Foo/`) instead of flat dotted siblings. Only on the `maildir` subcommand. |

When both sides set `--dovecot-keywords` and the dovecot slot table grows past 26 entries, the 27th keyword and onwards are dropped with a `warn!` log line per message; the conversion continues.

## Interfaces

- [io-email](https://github.com/pimalaya/io-email) provides the shared `EmailClientStd` and types (`Envelope`, `Mailbox`, `Flag`).
- Per-format I/O lives in [io-maildir](https://github.com/pimalaya/io-maildir) and [io-m2dir](https://github.com/pimalaya/io-m2dir).
- The spinner and the structured logger come from [pimalaya/cli](https://github.com/pimalaya/cli).

## Social

- IRC / Matrix space [#pimalaya](https://matrix.to/#/#pimalaya:matrix.org)
- Mastodon [@pimalaya](https://fosstodon.org/@pimalaya)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)
