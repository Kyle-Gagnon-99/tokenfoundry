use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use miette::{Result, miette};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
enum LogLevel {
    /// Log everything (including debug messages)
    Debug,
    /// Log informational messages
    Info,
    /// Log warnings
    Warning,
    /// Log errors only
    Error,
}

#[derive(Parser, Debug, Clone, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path to the configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// The log level to use (debug, info, warning, error)
    #[arg(short, long)]
    log_level: Option<LogLevel>,

    /// The command to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone, Deserialize, Serialize)]
enum Commands {
    /// Initialize a new tokenfoundry project in the current directory
    Init,
    /// Build the given outputs
    Build,
    /// Validates multiple portions of the project
    Check {
        /// Checks if the configuration is valid
        #[arg(long)]
        config: bool,
    },
    /// Allows you to inspect what token-foundry understands about your project
    Inspect {
        /// Allows you to inspect the given token or group by name
        #[arg(short, long)]
        token: Option<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_args = args.clone();

    let config_file = match args.config {
        Some(config_file) => config_file,
        None => {
            debug!("no configuration file specified, using default configuration");
            PathBuf::from("tokenfoundry.toml")
        }
    };

    if !config_file.exists() {
        return Err(miette!(
            "configuration file does not exist: {}",
            config_file.display()
        ));
    }

    // Run the appropriate command
    match args.command {
        Commands::Init => {
            debug!("initializing new tokenfoundry project in current directory");
        }
        Commands::Build => {
            debug!("building");
        }
        Commands::Check { config } => {
            debug!("checking configuration: {}", config);
        }
        Commands::Inspect { token } => {
            debug!("inspecting token: {:?}", token);
        }
    }

    Ok(())
}
