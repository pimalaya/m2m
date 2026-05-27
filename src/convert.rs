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

//! Conversion pipeline: lists mailboxes from the source, mirrors them on the
//! target, fans message copies out across a worker pool, and aggregates
//! per-mailbox outcomes into a [`Report`].

use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use anyhow::{Context as _, Result, anyhow, bail};
use crossbeam_queue::SegQueue;
use io_email::client::EmailClientStd;
use log::{info, trace};
use pimalaya_cli::spinner::Spinner;

use crate::{
    BackendSpec, Context,
    report::{MailboxOutcome, MessageFailure, Report},
};

/// Source → target conversion request. Built directly by the target
/// subcommand's `execute()` once both backend specs and the shared
/// context are known, then driven by [`Self::convert`].
pub struct Conversion {
    pub source: BackendSpec,
    pub target: BackendSpec,
    pub context: Context,
}

impl Conversion {
    /// Drives the full source → target conversion.
    pub fn convert(self) -> Result<Report> {
        let mut report = Report {
            dry_run: self.context.dry_run,
            ..Default::default()
        };

        let spinner = Spinner::start("Opening source");
        let mut source = self
            .source
            .build(&self.context.inbox, false)
            .context("Open source")?;
        spinner.success("Opened source");

        let spinner = Spinner::start("Listing source mailboxes");
        let mut source_names: Vec<String> = source
            .list_mailboxes(false)
            .context("List source mailboxes")?
            .into_iter()
            .map(|m| m.name)
            .collect();

        let filter =
            MailboxFilter::new(&self.context.include_mailbox, &self.context.exclude_mailbox);
        source_names.retain(|m| filter.accepts(m));
        spinner.success(format!("Listed {} source mailbox(es)", source_names.len()));

        let mappings = resolve_inbox(&source_names, &self.context.inbox)?;

        if self.context.dry_run {
            for m in &mappings {
                report.mailboxes.push(MailboxOutcome {
                    name: m.display_name(),
                    created: false,
                    messages_total: 0,
                    messages_copied: 0,
                    messages_skipped: 0,
                    failures: Vec::new(),
                });
            }
            return Ok(report);
        }

        let spinner = Spinner::start("Opening target");
        let mut target = self
            .target
            .build(&self.context.inbox, true)
            .context("Open target (initialising store if missing)")?;
        let existing_targets: BTreeSet<String> = target
            .list_mailboxes(false)
            .context("List target mailboxes")?
            .into_iter()
            .map(|m| m.name)
            .collect();
        spinner.success(format!(
            "Opened target ({} existing mailbox(es))",
            existing_targets.len()
        ));

        let spinner = Spinner::start("Creating target mailboxes");
        let mut created_flags = Vec::with_capacity(mappings.len());
        for m in &mappings {
            let exists = existing_targets.contains(&m.target);
            if !exists {
                target
                    .create_mailbox(&m.target)
                    .with_context(|| format!("Create target mailbox `{}`", m.target))?;
            }
            created_flags.push(!exists);
        }
        let created_count = created_flags.iter().filter(|c| **c).count();
        spinner.success(format!(
            "Mirrored {} mailbox(es) on target ({created_count} new)",
            mappings.len()
        ));

        let workers = self.context.workers.max(1);

        let spinner = Spinner::start("Copying messages");
        let total_copied = Arc::new(AtomicUsize::new(0));
        let total_skipped = Arc::new(AtomicUsize::new(0));
        let total_failed = Arc::new(AtomicUsize::new(0));

        for (idx, mapping) in mappings.iter().enumerate() {
            let copied = Arc::new(AtomicUsize::new(0));
            let skipped = Arc::new(AtomicUsize::new(0));
            let failures: Arc<Mutex<Vec<MessageFailure>>> = Arc::new(Mutex::new(Vec::new()));

            let envelopes = source
                .list_envelopes(&mapping.source, None, None, false)
                .with_context(|| format!("List envelopes in `{}`", mapping.source))?;

            let existing_target_ids = if self.context.skip_existing {
                target
                    .list_envelopes(&mapping.target, None, None, false)
                    .with_context(|| format!("List target envelopes in `{}`", mapping.target))?
                    .into_iter()
                    .map(|e| e.message_id.as_deref().unwrap_or(e.id.as_str()).to_string())
                    .collect::<BTreeSet<String>>()
            } else {
                BTreeSet::new()
            };

            let total = envelopes.len();
            info!(
                "[{}/{}] copying {} message(s) from `{}` to `{}`",
                idx + 1,
                mappings.len(),
                total,
                mapping.source,
                mapping.target,
            );

            let queue: Arc<SegQueue<EnvelopeTask>> = Arc::new(SegQueue::new());
            for env in envelopes {
                let dedup_key = env.message_id.clone().unwrap_or_else(|| env.id.clone());
                queue.push(EnvelopeTask {
                    id: env.id,
                    dedup_key,
                    flags: env.flags.into_iter().collect(),
                });
            }

            let spinner_ref = &spinner;
            thread::scope(|scope| -> Result<()> {
                let mut handles = Vec::with_capacity(workers);

                for _ in 0..workers {
                    let queue = queue.clone();
                    let copied = copied.clone();
                    let skipped = skipped.clone();
                    let failures = failures.clone();
                    let mapping = mapping.clone();
                    let source_spec = self.source.clone();
                    let target_spec = self.target.clone();
                    let inbox = self.context.inbox.clone();
                    let existing = existing_target_ids.clone();
                    let skip_existing = self.context.skip_existing;
                    let total_copied = total_copied.clone();
                    let total_skipped = total_skipped.clone();
                    let total_failed = total_failed.clone();

                    handles.push(scope.spawn(move || -> Result<()> {
                        let mut src = source_spec.build(&inbox, false)?;
                        let mut dst = target_spec.build(&inbox, false)?;

                        while let Some(task) = queue.pop() {
                            if skip_existing && existing.contains(&task.dedup_key) {
                                skipped.fetch_add(1, Ordering::Relaxed);
                                total_skipped.fetch_add(1, Ordering::Relaxed);
                            } else {
                                match copy_one(&mut src, &mut dst, &mapping, &task) {
                                    Ok(_) => {
                                        copied.fetch_add(1, Ordering::Relaxed);
                                        total_copied.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Err(err) => {
                                        total_failed.fetch_add(1, Ordering::Relaxed);
                                        let mut fails = failures.lock().unwrap();
                                        fails.push(MessageFailure {
                                            id: task.id.clone(),
                                            error: format!("{err:#}"),
                                        });
                                    }
                                }
                            }

                            spinner_ref.set_message(progress_message(
                                &total_copied,
                                &total_skipped,
                                &total_failed,
                            ));
                        }
                        Ok(())
                    }));
                }

                for handle in handles {
                    match handle.join() {
                        Ok(result) => result?,
                        Err(_) => bail!("worker thread panicked"),
                    }
                }

                Ok(())
            })?;

            let failures = Arc::try_unwrap(failures)
                .map_err(|_| anyhow!("failures arc still shared"))?
                .into_inner()
                .unwrap();

            report.mailboxes.push(MailboxOutcome {
                name: mapping.display_name(),
                created: created_flags[idx],
                messages_total: total,
                messages_copied: copied.load(Ordering::Relaxed),
                messages_skipped: skipped.load(Ordering::Relaxed),
                failures,
            });
        }

        spinner.success(progress_message(
            &total_copied,
            &total_skipped,
            &total_failed,
        ));

        Ok(report)
    }
}

/// Formats the running message counters for the copy spinner.
fn progress_message(copied: &AtomicUsize, skipped: &AtomicUsize, failed: &AtomicUsize) -> String {
    let copied = copied.load(Ordering::Relaxed);
    let skipped = skipped.load(Ordering::Relaxed);
    let failed = failed.load(Ordering::Relaxed);

    let mut parts = vec![format!("{copied} copied")];
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    parts.join(", ")
}

/// Source ↔ target mailbox name pair. The two are equal for most
/// folders; they only diverge when the source's inbox is renamed to
/// the canonical [`Context::inbox`] alias on the target.
#[derive(Clone, Debug)]
struct MailboxMapping {
    source: String,
    target: String,
}

impl MailboxMapping {
    fn display_name(&self) -> String {
        if self.source == self.target {
            self.source.clone()
        } else {
            format!("{} → {}", self.source, self.target)
        }
    }
}

/// Walks `source_names`, finds the inbox via case-insensitive match
/// against `inbox`, and renames it to `inbox` on the target. Folders
/// that don't look like the inbox pass through unchanged.
///
/// 0 inbox candidates: pass through. 1: rename. >1: require an exact
/// match against `inbox` to disambiguate; bail otherwise.
fn resolve_inbox(source_names: &[String], inbox: &str) -> Result<Vec<MailboxMapping>> {
    let inbox_lower = inbox.to_ascii_lowercase();
    let candidates: Vec<&String> = source_names
        .iter()
        .filter(|n| n.to_ascii_lowercase() == inbox_lower)
        .collect();

    let source_inbox: Option<String> = match candidates.as_slice() {
        [] => None,
        [only] => Some((*only).clone()),
        many => {
            let exact = many.iter().find(|n| n.as_str() == inbox).copied();
            match exact {
                Some(name) => Some(name.clone()),
                None => bail!(
                    "Multiple source folders match `{inbox}` case-insensitively ({}). \
		     Pass `--inbox` with the exact name to disambiguate.",
                    many.iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
    };

    Ok(source_names
        .iter()
        .map(|name| {
            let target = match &source_inbox {
                Some(s) if s == name => inbox.to_string(),
                _ => name.clone(),
            };

            MailboxMapping {
                source: name.clone(),
                target,
            }
        })
        .collect())
}

struct EnvelopeTask {
    id: String,
    dedup_key: String,
    flags: Vec<io_email::flag::Flag>,
}

fn copy_one(
    src: &mut EmailClientStd,
    dst: &mut EmailClientStd,
    mapping: &MailboxMapping,
    task: &EnvelopeTask,
) -> Result<()> {
    trace!(
        "copy `{}` from `{}` to `{}`",
        task.id, mapping.source, mapping.target
    );

    let bytes = src
        .get_message(&mapping.source, &task.id)
        .with_context(|| format!("Get message `{}` from source `{}`", task.id, mapping.source))?;

    dst.add_message(&mapping.target, &task.flags, bytes)
        .with_context(|| format!("Add message into target `{}`", mapping.target))?;

    Ok(())
}

struct MailboxFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl MailboxFilter {
    fn new(include: &[String], exclude: &[String]) -> Self {
        Self {
            include: include.iter().map(|s| s.to_ascii_lowercase()).collect(),
            exclude: exclude.iter().map(|s| s.to_ascii_lowercase()).collect(),
        }
    }

    fn accepts(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();

        if !self.include.is_empty() {
            return self.include.iter().any(|n| n == &lower);
        }

        !self.exclude.iter().any(|n| n == &lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn mapping(source: &str, target: &str) -> (String, String) {
        (source.to_string(), target.to_string())
    }

    fn pairs(mappings: &[MailboxMapping]) -> Vec<(String, String)> {
        mappings
            .iter()
            .map(|m| (m.source.clone(), m.target.clone()))
            .collect()
    }

    #[test]
    fn resolve_inbox_no_candidate_passes_everything_through() {
        let src = names(&["Sent", "Trash"]);
        let out = resolve_inbox(&src, "INBOX").unwrap();
        assert_eq!(
            pairs(&out),
            vec![mapping("Sent", "Sent"), mapping("Trash", "Trash")]
        );
    }

    #[test]
    fn resolve_inbox_single_match_renames_only_the_inbox() {
        let src = names(&["Inbox", "Sent", "Trash"]);
        let out = resolve_inbox(&src, "INBOX").unwrap();
        assert_eq!(
            pairs(&out),
            vec![
                mapping("Inbox", "INBOX"),
                mapping("Sent", "Sent"),
                mapping("Trash", "Trash"),
            ]
        );
    }

    #[test]
    fn resolve_inbox_single_exact_match_is_a_noop_rename() {
        let src = names(&["INBOX", "Sent"]);
        let out = resolve_inbox(&src, "INBOX").unwrap();
        assert_eq!(
            pairs(&out),
            vec![mapping("INBOX", "INBOX"), mapping("Sent", "Sent")]
        );
    }

    #[test]
    fn resolve_inbox_multiple_candidates_with_exact_match_picks_exact() {
        let src = names(&["inbox", "INBOX", "Inbox"]);
        let out = resolve_inbox(&src, "INBOX").unwrap();
        assert_eq!(
            pairs(&out),
            vec![
                mapping("inbox", "inbox"),
                mapping("INBOX", "INBOX"),
                mapping("Inbox", "Inbox"),
            ]
        );
    }

    #[test]
    fn resolve_inbox_multiple_candidates_without_exact_match_bails() {
        let src = names(&["inbox", "Inbox"]);
        let err = resolve_inbox(&src, "INBOX").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("Multiple source folders match"));
        assert!(msg.contains("`inbox`"));
        assert!(msg.contains("`Inbox`"));
    }

    #[test]
    fn mailbox_mapping_display_name_collapses_identity() {
        let m = MailboxMapping {
            source: "Sent".into(),
            target: "Sent".into(),
        };
        assert_eq!(m.display_name(), "Sent");
    }

    #[test]
    fn mailbox_mapping_display_name_shows_arrow_on_rename() {
        let m = MailboxMapping {
            source: "Inbox".into(),
            target: "INBOX".into(),
        };
        assert_eq!(m.display_name(), "Inbox → INBOX");
    }

    #[test]
    fn mailbox_filter_default_accepts_everything() {
        let f = MailboxFilter::new(&[], &[]);
        assert!(f.accepts("Inbox"));
        assert!(f.accepts("Trash"));
    }

    #[test]
    fn mailbox_filter_include_is_case_insensitive() {
        let f = MailboxFilter::new(&["INBOX".into(), "sent".into()], &[]);
        assert!(f.accepts("Inbox"));
        assert!(f.accepts("Sent"));
        assert!(!f.accepts("Trash"));
    }

    #[test]
    fn mailbox_filter_exclude_is_case_insensitive() {
        let f = MailboxFilter::new(&[], &["TRASH".into()]);
        assert!(f.accepts("Inbox"));
        assert!(!f.accepts("Trash"));
        assert!(!f.accepts("trash"));
    }

    #[test]
    fn mailbox_filter_include_overrides_exclude() {
        let f = MailboxFilter::new(&["Inbox".into()], &["Inbox".into()]);
        assert!(f.accepts("Inbox"));
        assert!(!f.accepts("Sent"));
    }
}
