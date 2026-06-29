use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use figment::{
    Figment,
    providers::{self, Format, Serialized, Toml},
};
use miette::{IntoDiagnostic, Result, miette};
use serde::{Deserialize, Serialize};
use tokenfoundry_core::{
    config::LogLevel as CoreLogLevel,
    config::{BaseConfigMeta, Config},
};
use tracing::debug;
use tracing_subscriber::EnvFilter;

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

impl From<LogLevel> for CoreLogLevel {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Debug => CoreLogLevel::Debug,
            LogLevel::Info => CoreLogLevel::Info,
            LogLevel::Warning => CoreLogLevel::Warning,
            LogLevel::Error => CoreLogLevel::Error,
        }
    }
}

fn log_level_to_filter_directive(log_level: CoreLogLevel) -> &'static str {
    match log_level {
        CoreLogLevel::Debug => "debug",
        CoreLogLevel::Info => "info",
        CoreLogLevel::Warning => "warn",
        CoreLogLevel::Error => "error",
    }
}

fn init_logging(log_level: CoreLogLevel) -> Result<()> {
    let default_directive = log_level_to_filter_directive(log_level);

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_directive))
        .into_diagnostic()?;

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init()
        .map_err(|e| miette!("failed to initialize logging subscriber: {e}"))?;

    Ok(())
}

fn build_extended_config(config_file: &Path) -> Figment {
    let mut figment = Figment::from(Serialized::defaults(Config::default()));

    // Peek inside the current file using a lightweight figment
    let peek_figment = Figment::new().merge(Toml::file(config_file));

    if let Ok(meta) = peek_figment.extract::<BaseConfigMeta>() {
        if let Some(parent_path) = meta.extends {
            let resolved_parent = if parent_path.is_relative() {
                config_file
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(parent_path)
            } else {
                parent_path
            };
            figment = figment.merge(Toml::file(resolved_parent));
        }
    }
    figment.merge(Toml::file(config_file))
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

    // Next, we allow configuration overriding and extending. We defined an "extends" field in the configuration file, which allows us to specify
    // another configuration file to extend from.
    let config: Config = build_extended_config(&config_file)
        .merge(providers::Env::prefixed("TF_"))
        .merge(Serialized::defaults(config_args))
        .extract()
        .expect("Failed to parse configuration");

    // CLI log level wins, otherwise use the config value, otherwise fall back to the core default.
    let log_level = args
        .log_level
        .map(Into::into)
        .or(config.log_level)
        .unwrap_or(CoreLogLevel::Info);

    init_logging(log_level)?;
    debug!("final configuration: {:#?}", config);

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
