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

//! Binary entry point: parses the CLI, configures logging and resolves the
//! global flags into a [`Context`] before letting the source subcommand drive
//! the rest of the chain via `execute()`.

#[cfg(any(feature = "maildir", feature = "m2dir"))]
mod convert;
#[cfg(feature = "m2dir")]
mod m2dir;
#[cfg(feature = "maildir")]
mod maildir;
#[cfg(feature = "maildir")]
mod maildirpp;
mod report;

use std::thread;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
#[cfg(any(feature = "maildir", feature = "m2dir"))]
use io_email::client::EmailClientStd;
use pimalaya_cli::{
    clap::{
        args::{JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
    },
    error::ErrorReport,
    log::Logger,
    long_version,
    printer::{Printer, StdoutPrinter},
};

#[cfg(feature = "m2dir")]
use crate::m2dir::{M2dirSourceCommand, M2dirSpec, M2dirTargetCommand};
#[cfg(feature = "maildir")]
use crate::maildir::{MaildirSourceCommand, MaildirSpec, MaildirTargetCommand};
#[cfg(feature = "maildir")]
use crate::maildirpp::{MaildirPlusSourceCommand, MaildirPlusSpec, MaildirPlusTargetCommand};
use crate::report::Report;

fn main() {
    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);
    let result = execute(cli, &mut printer);
    ErrorReport::eval(&mut printer, result);
}

fn execute(cli: Cli, printer: &mut impl Printer) -> Result<()> {
    Logger::try_init(&cli.log)?;

    let workers = cli
        .workers
        .or_else(|| thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(4)
        .clamp(1, 16);

    let context = Context {
        workers,
        inbox: cli.inbox.unwrap_or_else(|| String::from("INBOX")),
        include_mailbox: cli.include_mailbox,
        exclude_mailbox: cli.exclude_mailbox,
        skip_existing: cli.skip_existing,
        dry_run: cli.dry_run,
    };

    cli.cmd.execute(context, printer)
}

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true)]
#[command(subcommand_required = true, infer_subcommands = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Worker thread count. Defaults to the available parallelism,
    /// capped at 16.
    #[arg(long, short = 'w', value_name = "N", global = true)]
    pub workers: Option<usize>,

    /// Canonical inbox folder name. Used to detect the inbox on the
    /// source (case-insensitive match) and to name it on the target.
    /// Defaults to `INBOX`.
    #[arg(long, value_name = "NAME", global = true)]
    pub inbox: Option<String>,

    /// Mailbox name to include (repeatable, case-insensitive).
    #[arg(long, value_name = "NAME", global = true)]
    pub include_mailbox: Vec<String>,

    /// Mailbox name to exclude (repeatable, case-insensitive).
    #[arg(long, value_name = "NAME", global = true)]
    pub exclude_mailbox: Vec<String>,

    /// Skip messages already present in the target.
    #[arg(long, global = true)]
    pub skip_existing: bool,

    /// List the conversion plan without writing anything.
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(flatten)]
    pub json: JsonFlag,

    #[command(flatten)]
    pub log: LogFlags,

    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "lower")]
pub enum Command {
    /// Read from a standard Maildir tree.
    #[cfg(feature = "maildir")]
    Maildir(MaildirSourceCommand),
    /// Read from a Maildir++ tree (dotted folder siblings).
    #[cfg(feature = "maildir")]
    #[command(name = "maildir++")]
    MaildirPlus(MaildirPlusSourceCommand),
    /// Read from an m2dir tree.
    #[cfg(feature = "m2dir")]
    M2dir(M2dirSourceCommand),
    /// Generate shell completion scripts.
    Completions(CompletionCommand),
    /// Generate man pages.
    Manuals(ManualCommand),
}

impl Command {
    #[cfg_attr(
        not(any(feature = "maildir", feature = "m2dir")),
        allow(unused_variables)
    )]
    pub fn execute(self, ctx: Context, printer: &mut impl Printer) -> Result<()> {
        match self {
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => printer.out(cmd.execute(ctx)?),
            #[cfg(feature = "maildir")]
            Self::MaildirPlus(cmd) => printer.out(cmd.execute(ctx)?),
            #[cfg(feature = "m2dir")]
            Self::M2dir(cmd) => printer.out(cmd.execute(ctx)?),
            Self::Completions(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manuals(cmd) => cmd.execute(printer, Cli::command()),
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "lower")]
pub enum Target {
    /// Write to a standard Maildir tree.
    #[cfg(feature = "maildir")]
    Maildir(MaildirTargetCommand),
    /// Write to a Maildir++ tree (dotted folder siblings).
    #[cfg(feature = "maildir")]
    #[command(name = "maildir++")]
    MaildirPlus(MaildirPlusTargetCommand),
    /// Write to an m2dir tree.
    #[cfg(feature = "m2dir")]
    M2dir(M2dirTargetCommand),
}

#[cfg(any(feature = "maildir", feature = "m2dir"))]
impl Target {
    pub fn execute(self, source: BackendSpec, ctx: Context) -> Result<Report> {
        match self {
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => cmd.execute(source, ctx),
            #[cfg(feature = "maildir")]
            Self::MaildirPlus(cmd) => cmd.execute(source, ctx),
            #[cfg(feature = "m2dir")]
            Self::M2dir(cmd) => cmd.execute(source, ctx),
        }
    }
}

/// Resolved global flags handed down through the subcommand chain.
#[derive(Clone, Debug)]
pub struct Context {
    pub workers: usize,
    pub inbox: String,
    pub include_mailbox: Vec<String>,
    pub exclude_mailbox: Vec<String>,
    pub skip_existing: bool,
    pub dry_run: bool,
}

/// Bridge between the CLI commands and [`crate::convert::Conversion`].
/// Each variant wraps the spec owned by its respective backend module;
/// [`Self::build`] dispatches to that spec's `build`.
#[cfg(any(feature = "maildir", feature = "m2dir"))]
#[derive(Clone, Debug)]
pub enum BackendSpec {
    #[cfg(feature = "maildir")]
    Maildir(MaildirSpec),
    #[cfg(feature = "maildir")]
    MaildirPlus(MaildirPlusSpec),
    #[cfg(feature = "m2dir")]
    M2dir(M2dirSpec),
}

#[cfg(any(feature = "maildir", feature = "m2dir"))]
impl BackendSpec {
    /// Builds an [`EmailClientStd`]. `inbox` configures the
    /// `maildirpp_inbox` alias used by both Maildir-family backends to
    /// name the root mailbox; m2dir ignores it. `init = true` creates
    /// the on-disk store if missing (target side).
    pub fn build(&self, inbox: &str, init: bool) -> Result<EmailClientStd> {
        match self {
            #[cfg(feature = "maildir")]
            Self::Maildir(s) => s.build(inbox, init),
            #[cfg(feature = "maildir")]
            Self::MaildirPlus(s) => s.build(inbox, init),
            #[cfg(feature = "m2dir")]
            Self::M2dir(s) => {
                let _ = inbox;
                s.build(init)
            }
        }
    }
}
