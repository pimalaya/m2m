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

//! End-of-run conversion report. Aggregates per-mailbox outcomes and
//! per-message failures, then renders a neverest-style breakdown (mailbox
//! patch, warnings, errors, final status line).

use std::fmt;

use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub dry_run: bool,
    pub mailboxes: Vec<MailboxOutcome>,
    /// Failures not tied to a specific message (e.g. listing,
    /// mailbox-create errors that didn't abort the run).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MailboxOutcome {
    pub name: String,
    pub created: bool,
    pub messages_total: usize,
    pub messages_copied: usize,
    pub messages_skipped: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<MessageFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageFailure {
    pub id: String,
    pub error: String,
}

impl Report {
    pub fn totals(&self) -> Totals {
        let mut t = Totals::default();
        t.mailboxes = self.mailboxes.len();
        for m in &self.mailboxes {
            t.created += usize::from(m.created);
            t.messages_total += m.messages_total;
            t.copied += m.messages_copied;
            t.skipped += m.messages_skipped;
            t.failed += m.failures.len();
        }
        t
    }
}

#[derive(Debug, Default)]
pub struct Totals {
    pub mailboxes: usize,
    pub created: usize,
    pub messages_total: usize,
    pub copied: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;

        let totals = self.totals();

        if !self.mailboxes.is_empty() {
            writeln!(f, "Mailboxes ({}):", self.mailboxes.len())?;

            for m in &self.mailboxes {
                let tag = if m.created { " (new)" } else { "" };
                let mut parts = vec![format!("{} ok", m.messages_copied)];

                if m.messages_skipped > 0 {
                    parts.push(format!("{} skipped", m.messages_skipped));
                }

                if !m.failures.is_empty() {
                    parts.push(format!("{} failed", m.failures.len()));
                }

                writeln!(
                    f,
                    " - {name}{tag}: {summary} / {total}",
                    name = m.name,
                    summary = parts.join(", "),
                    total = m.messages_total
                )?;
            }

            writeln!(f)?;
        }

        if !self.warnings.is_empty() {
            writeln!(f, "Warnings ({}):", self.warnings.len())?;

            for w in &self.warnings {
                writeln!(f, " - {w}")?;
            }

            writeln!(f)?;
        }

        if totals.failed > 0 {
            writeln!(f, "Errors ({}):", totals.failed)?;

            for m in &self.mailboxes {
                for fail in &m.failures {
                    writeln!(
                        f,
                        " - `{mailbox}` `{id}`: {err}",
                        mailbox = m.name,
                        id = fail.id,
                        err = fail.error
                    )?;
                }
            }

            writeln!(f)?;
        }

        let Totals {
            mailboxes,
            created,
            messages_total: _,
            copied,
            skipped,
            failed,
        } = totals;

        match (self.dry_run, failed) {
            (true, _) => write!(
                f,
                "Dry run: would convert {copied} message(s) across \
		 {mailboxes} mailbox(es) ({created} new)"
            ),
            (false, 0) => write!(
                f,
                "Converted {copied} message(s) across \
		 {mailboxes} mailbox(es) ({created} new, {skipped} skipped)"
            ),
            (false, _) => write!(
                f,
                "Partially converted: {copied} ok, {failed} failed, \
		 {skipped} skipped across {mailboxes} mailbox(es)"
            ),
        }
    }
}
