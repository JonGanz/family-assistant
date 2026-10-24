use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use fa_domain::{CalendarSourceId, DeviceId, HouseholdId};

#[derive(Debug, Parser)]
#[command(
    name = "fa-server",
    version,
    about = "Family Assistant server and operator CLI"
)]
pub struct Cli {
    /// Household to operate on (only needed when the database holds more than one).
    #[arg(long, global = true)]
    pub household: Option<HouseholdId>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ServeMode {
    /// HTTP API (runs migrations at startup).
    Api,
    /// Job worker and scheduler only.
    Worker,
    /// Both in one process.
    All,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the server.
    Serve {
        #[arg(value_enum, default_value = "all")]
        mode: ServeMode,
    },
    /// Apply database migrations and exit.
    Migrate,
    /// Import or export the household TOML.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage login accounts.
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// Manage dashboard devices.
    Device {
        #[command(subcommand)]
        action: DeviceAction,
    },
    /// Manage adult PINs.
    Pin {
        #[command(subcommand)]
        action: PinAction,
    },
    /// Calendar tools.
    Calendar {
        #[command(subcommand)]
        action: CalendarAction,
    },
    /// Weather tools.
    Weather {
        #[command(subcommand)]
        action: WeatherAction,
    },
    /// Write the OpenAPI document.
    GenOpenapi {
        #[arg(long, default_value = "apps/web/src/gen/openapi.json")]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    Import {
        file: PathBuf,
        /// Archive keyed members the file no longer lists.
        #[arg(long)]
        replace: bool,
    },
    Export {
        /// Write to a file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum UserAction {
    Add {
        /// Member key from the household TOML.
        #[arg(long)]
        member: String,
        #[arg(long)]
        username: String,
        /// Read the password from the first line of stdin instead of prompting.
        #[arg(long)]
        password_stdin: bool,
    },
    ResetPassword {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_stdin: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeviceAction {
    Pair {
        #[arg(long)]
        name: String,
    },
    Revoke {
        id: DeviceId,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarAction {
    /// Sync one source now, in this process, and print what changed.
    Sync { source: CalendarSourceId },
}

#[derive(Debug, Subcommand)]
pub enum WeatherAction {
    /// Fetch the forecast, evaluate the rules in this process and print the result.
    Fetch,
}

#[derive(Debug, Subcommand)]
pub enum PinAction {
    Set {
        /// Member key of an adult.
        #[arg(long)]
        member: String,
        /// Read the PIN from the first line of stdin instead of prompting.
        #[arg(long)]
        pin_stdin: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn cli_is_consistent() {
        super::Cli::command().debug_assert();
    }
}
