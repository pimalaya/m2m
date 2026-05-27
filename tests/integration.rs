// This file is part of m2m, a CLI to convert from/to filesystem-based
// mailboxes.
//
// Copyright (C) 2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! End-to-end CLI tests: spawn the built `m2m` binary against
//! tempfile-backed Maildir / Maildir++ stores and assert the resulting
//! filesystem layout. The binary path is provided by Cargo via
//! `CARGO_BIN_EXE_m2m`.

#![cfg(all(feature = "maildir", feature = "m2dir"))]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

const M2M: &str = env!("CARGO_BIN_EXE_m2m");

/// Lays out an empty Maildir (`cur` / `new` / `tmp`) at `path`.
fn make_maildir(path: &Path) {
    fs::create_dir_all(path.join("cur")).unwrap();
    fs::create_dir_all(path.join("new")).unwrap();
    fs::create_dir_all(path.join("tmp")).unwrap();
}

/// Writes a single message file directly into `<maildir>/cur/`.
fn write_message(maildir: &Path, name: &str, raw: &[u8]) {
    fs::write(maildir.join("cur").join(name), raw).unwrap();
}

/// Counts message files (files only, no subdirs) under `<maildir>/cur/`.
fn count_cur(maildir: &Path) -> usize {
    let cur = maildir.join("cur");
    if !cur.exists() {
        return 0;
    }
    fs::read_dir(&cur)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_type().unwrap().is_file())
        .count()
}

/// Returns the byte content of the single message in `<maildir>/cur/`.
/// Panics if there isn't exactly one.
fn read_only_message(maildir: &Path) -> Vec<u8> {
    let cur = maildir.join("cur");
    let files: Vec<_> = fs::read_dir(&cur)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            if e.file_type().unwrap().is_file() {
                Some(e.path())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        files.len(),
        1,
        "expected exactly one message in {}",
        cur.display()
    );
    fs::read(&files[0]).unwrap()
}

/// Returns `(tmpdir, src, dst)` with `src` populated by `setup`.
fn workspace(setup: impl FnOnce(&Path)) -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    setup(&src);
    (tmp, src, dst)
}

const SAMPLE_RAW: &[u8] = b"From: alice@example.org\r\n\
To: bob@example.org\r\n\
Date: Tue, 15 Apr 1994 08:12:31 GMT\r\n\
Subject: hi\r\n\
Message-ID: <demo-1@example.org>\r\n\
\r\n\
hello\r\n";

#[test]
fn maildir_to_maildir_preserves_inbox_with_no_rename() {
    // Source layout: src/INBOX with one message; --inbox defaults to
    // INBOX so the case-sensitive match is identity and no rename
    // happens.
    let (_tmp, src, dst) = workspace(|src| {
        make_maildir(&src.join("INBOX"));
        write_message(&src.join("INBOX"), "1700000000.abc:2,S", SAMPLE_RAW);
    });

    let out = Command::new(M2M)
        .arg("maildir")
        .arg(&src)
        .arg("maildir")
        .arg(&dst)
        .output()
        .expect("spawn m2m");
    assert!(
        out.status.success(),
        "m2m failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(count_cur(&dst.join("INBOX")), 1);
    let bytes = read_only_message(&dst.join("INBOX"));
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("Subject: hi"));
    assert!(text.contains("Message-ID: <demo-1@example.org>"));
}

#[test]
fn maildir_to_maildir_renames_source_inbox_to_canonical_alias() {
    // Source folder is "Inbox" (mixed case). With default --inbox
    // INBOX, the case-insensitive lookup finds "Inbox" and renames it
    // to "INBOX" on the target.
    let (_tmp, src, dst) = workspace(|src| {
        make_maildir(&src.join("Inbox"));
        write_message(&src.join("Inbox"), "1700000001.def:2,", SAMPLE_RAW);
    });

    let out = Command::new(M2M)
        .arg("maildir")
        .arg(&src)
        .arg("maildir")
        .arg(&dst)
        .output()
        .expect("spawn m2m");
    assert!(
        out.status.success(),
        "m2m failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(count_cur(&dst.join("INBOX")), 1, "expected INBOX on target");
    assert!(
        !dst.join("Inbox").exists(),
        "source-cased folder must not appear on target"
    );
}

#[test]
fn maildir_dry_run_does_not_create_target() {
    let (_tmp, src, dst) = workspace(|src| {
        make_maildir(&src.join("INBOX"));
        write_message(&src.join("INBOX"), "1700000002.ghi:2,", SAMPLE_RAW);
    });

    let out = Command::new(M2M)
        .arg("--dry-run")
        .arg("maildir")
        .arg(&src)
        .arg("maildir")
        .arg(&dst)
        .output()
        .expect("spawn m2m");
    assert!(
        out.status.success(),
        "m2m failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !dst.exists(),
        "target path must not be created in --dry-run mode"
    );
}

#[test]
fn maildir_to_m2dir_round_trip() {
    let (_tmp, src, dst) = workspace(|src| {
        make_maildir(&src.join("INBOX"));
        write_message(&src.join("INBOX"), "1700000003.jkl:2,", SAMPLE_RAW);
    });

    let out = Command::new(M2M)
        .arg("maildir")
        .arg(&src)
        .arg("m2dir")
        .arg(&dst)
        .output()
        .expect("spawn m2m");
    assert!(
        out.status.success(),
        "m2m failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The m2dir target should at minimum carry its store marker.
    assert!(
        dst.join(".m2store").exists(),
        "m2dir target must have the `.m2store` marker"
    );
}

#[test]
fn missing_subcommand_exits_with_help() {
    let out = Command::new(M2M).output().expect("spawn m2m");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Usage:") || stderr.contains("USAGE"),
        "expected usage hint in stderr, got: {stderr}"
    );
}
