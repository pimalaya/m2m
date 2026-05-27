# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release of the `m2m` CLI.
- Chained-subcommand grammar `m2m <SOURCE-CMD> <PATH> [src-flags] <TARGET-CMD> <PATH> [tgt-flags]` with backends `maildir`, `maildir++` and `m2dir`.
- Parallel copy loop backed by a `crossbeam_queue::SegQueue` and one worker pool per mailbox, sized via `--workers` (default = available parallelism, capped at 16).
- Idempotent re-runs via `--skip-existing` (dedup on `Message-ID`, falls back to the source id).
- `--dry-run` mode that lists the conversion plan without touching the target store.
- Per-side knobs `--dovecot-keywords`, `--keywords-header <x-keywords|x-label>`, `--strip-headers <CSV>` and `--fs-layout` for round-tripping custom Maildir keywords (dovecot slot file or body header) and selecting Dovecot fs layout.
- Global `--inbox <NAME>` (default `INBOX`): case-insensitively identifies the source inbox folder and renames it to the canonical alias on the target; Maildir++ root surfaces under the same alias.
- Maildir++ layout awareness: source / target backend `maildir++` enumerates and creates dotted folder siblings while reporting them as logical `Work/Foo` mailboxes; root maildir is surfaced as the inbox.
- Structured end-of-run report with per-mailbox `ok / skipped / failed` breakdown, dedicated `Errors` section (one entry per failed message), and a final status line. Renders as `--json` when requested.
- Automatic target initialisation on the first run (`Maildir` root created, `m2dir` `.m2store` marker written).
