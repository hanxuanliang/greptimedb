// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(assert_matches, let_chains)]

use async_trait::async_trait;
use clap::arg;
use common_telemetry::{error, info};

pub mod cli;
pub mod datanode;
pub mod error;
pub mod frontend;
pub mod metasrv;
pub mod options;
pub mod standalone;

lazy_static::lazy_static! {
    static ref APP_VERSION: prometheus::IntGaugeVec =
        prometheus::register_int_gauge_vec!("greptime_app_version", "app version", &["short_version", "version"]).unwrap();
}

#[async_trait]
pub trait App: Send {
    fn name(&self) -> &str;

    /// A hook for implementor to make something happened before actual startup. Defaults to no-op.
    async fn pre_start(&mut self) -> error::Result<()> {
        Ok(())
    }

    async fn start(&mut self) -> error::Result<()>;

    async fn stop(&self) -> error::Result<()>;
}

pub async fn start_app(mut app: Box<dyn App>) -> error::Result<()> {
    info!("Starting app: {}", app.name());

    app.pre_start().await?;

    app.start().await?;

    if let Err(e) = tokio::signal::ctrl_c().await {
        error!("Failed to listen for ctrl-c signal: {}", e);
        // It's unusual to fail to listen for ctrl-c signal, maybe there's something unexpected in
        // the underlying system. So we stop the app instead of running nonetheless to let people
        // investigate the issue.
    }

    app.stop().await?;
    info!("Goodbye!");
    Ok(())
}

/// Log the versions of the application, and the arguments passed to the cli.
/// `version_string` should be the same as the output of cli "--version";
/// and the `app_version` is the short version of the codes, often consist of git branch and commit.
pub fn log_versions(version_string: &str, app_version: &str) {
    // Report app version as gauge.
    APP_VERSION
        .with_label_values(&[env!("CARGO_PKG_VERSION"), app_version])
        .inc();

    // Log version and argument flags.
    info!("GreptimeDB version: {}", version_string);

    log_env_flags();
}

pub fn greptimedb_cli() -> clap::Command {
    let cmd = clap::Command::new("greptimedb").subcommand_required(true);

    #[cfg(feature = "tokio-console")]
    let cmd = cmd.arg(arg!(--"tokio-console-addr"[TOKIO_CONSOLE_ADDR]));

    cmd.args([arg!(--"log-dir"[LOG_DIR]), arg!(--"log-level"[LOG_LEVEL])])
}

fn log_env_flags() {
    info!("command line arguments");
    for argument in std::env::args() {
        info!("argument: {}", argument);
    }
}
