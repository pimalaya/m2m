# m2m [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya)

CLI to convert filesystem-based mailboxes, written in Rust.

> [!CAUTION]
> m2m is in active development and shipped as `v0.x.x`. Expect breaking changes between releases until stabilisation.

## Table of contents

- [Features](#features)
- [Installation](#installation)
  - [Cargo](#cargo)
  - [Nix](#nix)
  - [Sources](#sources)
- [Usage](#usage)
- [Quirks](#quirks)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- Convert from/to **Maildir** <sup>[specs](https://cr.yp.to/proto/maildir.html)</sup>, **Maildir++** <sup>[specs](https://www.courier-mta.org/imap/README.maildirquota.html)</sup> and **m2dir** <sup>[specs](https://man.sr.ht/~bitfehler/m2dir/)</sup> filesystem stores.
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

## Quirks

`maildir` accepts four optional flags on either side, `maildir++` accepts three (no `--fs-layout`), `m2dir` accepts none.

| Flag | What it does |
| --- | --- |
| `--dovecot-keywords` | Reads / writes the per-folder `dovecot-keywords` sidecar (Dovecot / mbsync convention). Custom keywords in filenames (`a..z`) round-trip through the slot table. |
| `--keywords-header <x-keywords\|x-label>` | Mirrors custom keywords into the chosen body header. `x-keywords` follows the OfflineIMAP / mbsync convention (comma-separated); `x-label` follows the mutt / notmuch convention (space-separated). |
| `--strip-headers <CSV>` | Drops these header names from each message on read; useful to clean Mozilla / Pegasus markers (`X-Mozilla-Status`, `X-Status`, ...). |
| `--fs-layout` | Dovecot fs layout: subfolders are nested filesystem directories (`Work/Foo/`) instead of flat dotted siblings. Only on the `maildir` subcommand. |

When both sides set `--dovecot-keywords` and the dovecot slot table grows past 26 entries, the 27th keyword and onwards are dropped with a `warn!` log line per message; the conversion continues.

## Social

- Chat on [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- News on [Mastodon](https://fosstodon.org/@pimalaya) or [RSS](https://fosstodon.org/@pimalaya.rss)
- Mail at [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Special thanks to the [NLnet foundation](https://nlnet.nl/) and the [European Commission](https://www.ngi.eu/) that have been financially supporting the project for years:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 in preparation…*

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
