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

//! CLI surface, backend spec and client builder for the `m2dir` source and
//! target subcommands.

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::Args;
use io_email::client::EmailClientStd;
use io_m2dir::client::M2dirClient;

use crate::{BackendSpec, Context, Target, convert::Conversion, report::Report};

/// Convert from an m2dir source tree.
#[derive(Args, Debug)]
pub struct M2dirSourceCommand {
    /// Filesystem root of the source m2dir store (the directory
    /// containing the `.m2store` marker).
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    #[command(subcommand)]
    pub target: Target,
}

impl M2dirSourceCommand {
    pub fn execute(self, ctx: Context) -> Result<Report> {
        let source = BackendSpec::M2dir(M2dirSpec { path: self.path });
        self.target.execute(source, ctx)
    }
}

/// Convert to an m2dir target tree.
#[derive(Args, Debug)]
pub struct M2dirTargetCommand {
    /// Filesystem root of the target m2dir store; the `.m2store`
    /// marker is created on first run.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,
}

impl M2dirTargetCommand {
    pub fn execute(self, source: BackendSpec, ctx: Context) -> Result<Report> {
        let target = BackendSpec::M2dir(M2dirSpec { path: self.path });

        let conv = Conversion {
            source,
            target,
            context: ctx,
        };

        conv.convert()
    }
}

/// Resolved spec for an m2dir backend. Built by the CLI commands
/// above; consumed by [`BackendSpec::build`].
#[derive(Clone, Debug)]
pub struct M2dirSpec {
    pub path: PathBuf,
}

impl M2dirSpec {
    pub fn build(&self, init: bool) -> Result<EmailClientStd> {
        let client = M2dirClient::new(self.path.to_string_lossy().into_owned());

        if init {
            client
                .init_store()
                .with_context(|| format!("Initialise m2store at {}", self.path.display()))?;
        }

        Ok(EmailClientStd::from(client))
    }
}
