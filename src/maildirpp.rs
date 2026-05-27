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

//! CLI surface, backend spec and client builder for the `maildir++` source and
//! target subcommands.

use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result};
use clap::Args;
use io_email::client::EmailClientStd;
use io_maildir::{client::MaildirClient, flag::KeywordHeader};

use crate::{
    BackendSpec, Context, Target, convert::Conversion, maildir::KeywordHeaderArg, report::Report,
};

#[derive(Args, Debug)]
pub struct MaildirPlusSourceCommand {
    /// Filesystem root of the source Maildir++ store. Subfolders are
    /// read as flat dotted siblings (`.Work.Foo/`) and surfaced as
    /// logical `Work/Foo` mailboxes; the root itself is the inbox.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Resolve custom keywords through the per-folder
    /// `dovecot-keywords` file (Dovecot / mbsync convention).
    #[arg(long)]
    pub dovecot_keywords: bool,

    /// Mirror custom keywords from the chosen body header on read.
    #[arg(long, value_name = "HEADER", value_enum)]
    pub keywords_header: Option<KeywordHeaderArg>,

    /// Comma-separated list of header names to remove from each
    /// message on read.
    #[arg(long, value_name = "CSV", value_delimiter = ',')]
    pub strip_headers: Vec<String>,

    #[command(subcommand)]
    pub target: Target,
}

impl MaildirPlusSourceCommand {
    pub fn execute(self, ctx: Context) -> Result<Report> {
        let source = BackendSpec::MaildirPlus(MaildirPlusSpec {
            path: self.path,
            dovecot_keywords: self.dovecot_keywords,
            keywords_header: self.keywords_header.map(Into::into),
            strip_headers: self.strip_headers,
        });
        self.target.execute(source, ctx)
    }
}

#[derive(Args, Debug)]
pub struct MaildirPlusTargetCommand {
    /// Filesystem root of the target Maildir++ store; created if
    /// missing. Subfolders are written as flat dotted siblings
    /// (`.Work.Foo/`); the inbox is written to the root.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Persist custom keywords through the per-folder
    /// `dovecot-keywords` file (Dovecot / mbsync convention).
    #[arg(long)]
    pub dovecot_keywords: bool,

    /// Inject custom keywords into the chosen body header on write.
    #[arg(long, value_name = "HEADER", value_enum)]
    pub keywords_header: Option<KeywordHeaderArg>,

    /// Comma-separated list of header names to remove from each
    /// message before writing.
    #[arg(long, value_name = "CSV", value_delimiter = ',')]
    pub strip_headers: Vec<String>,
}

impl MaildirPlusTargetCommand {
    pub fn execute(self, source: BackendSpec, ctx: Context) -> Result<Report> {
        let target = BackendSpec::MaildirPlus(MaildirPlusSpec {
            path: self.path,
            dovecot_keywords: self.dovecot_keywords,
            keywords_header: self.keywords_header.map(Into::into),
            strip_headers: self.strip_headers,
        });

        let conv = Conversion {
            source,
            target,
            context: ctx,
        };

        conv.convert()
    }
}

/// Resolved spec for a Maildir++ backend. Built by the CLI commands
/// above; consumed by [`BackendSpec::build`].
#[derive(Clone, Debug)]
pub struct MaildirPlusSpec {
    pub path: PathBuf,
    pub dovecot_keywords: bool,
    pub keywords_header: Option<KeywordHeader>,
    pub strip_headers: Vec<String>,
}

impl MaildirPlusSpec {
    pub fn build(&self, inbox: &str, init: bool) -> Result<EmailClientStd> {
        if init {
            fs::create_dir_all(&self.path)
                .with_context(|| format!("Create Maildir++ root at {}", self.path.display()))?;
        }

        let mut client = MaildirClient::new(self.path.to_string_lossy().into_owned());
        client.maildir_plus = true;
        client.fs_layout = false;
        client.maildirpp_inbox = inbox.to_string();
        client.dovecot_keywords = self.dovecot_keywords;
        client.keywords_header = self.keywords_header;
        client.strip_headers = self.strip_headers.clone();

        Ok(EmailClientStd::from(client))
    }
}
