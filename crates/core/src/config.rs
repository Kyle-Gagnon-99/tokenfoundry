//! The `config` module defines the configuration options to be found in the `tokenfoundry.toml` file.
//! This file is used to configure the behavior of the `tokenfoundry` CLI tool, such as the log level and other settings.
//! A configuration file allows users to configure the behavior of the CLI tool without having to specify command-line arguments every
//! time they run the tool.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub enum LogLevel {
    /// Log everything (including debug messages)
    Debug,
    /// Log informational messages
    Info,
    /// Log warnings
    Warning,
    /// Log errors only
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct BaseConfigMeta {
    /// A path to a configuration file to extend from. This allows you to have a base configuration file that can be extended by other configuration files, which is useful for sharing common configuration options across multiple projects or environments.
    pub extends: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TailwindConfig {
    /// Whether to enable transforming tokens to Tailwind CSS custom properties. If enabled, tokens will be transformed to Tailwind CSS custom properties (e.g., a token named "color-primary" would be transformed to "--color-primary"). If not enabled, tokens will not be transformed to Tailwind CSS custom properties.
    pub enabled: Option<bool>,
    /// The prefix to use for Tailwind CSS custom properties. For example, if the prefix is "tf", then a token named "color-primary" would be transformed to "--tf-color-primary".
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Config {
    /// A path to a configuration file to extend from. This allows you to have a base configuration file that can be extended by other configuration files, which is useful for sharing common configuration options across multiple projects or environments.
    pub extends: Option<PathBuf>,

    /// The log level to use for the CLI tool. This allows you to configure how much information the CLI tool logs to the console when it runs. For example, if the log level is set to "info", then the CLI tool will log informational messages, warnings, and errors, but not debug messages.
    #[serde(rename = "logLevel")]
    pub log_level: Option<LogLevel>,

    /// The Tailwind CSS configuration options. This allows you to configure how tokens are transformed to Tailwind CSS custom properties.
    pub tailwind: Option<TailwindConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            extends: None,
            log_level: Some(LogLevel::Info),
            tailwind: None,
        }
    }
}
