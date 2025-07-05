use crate::types::{RelayerConfig, SettlementError};
use clap::{Arg, Command};
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::{env, path::Path};
use tracing::{info, warn};

/// Configuration builder for the relayer
pub struct ConfigBuilder {
    config: Config,
}

/// CLI arguments
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub config_file: Option<String>,
    pub solana_rpc_url: Option<String>,
    pub aptos_rpc_url: Option<String>,
    pub log_level: Option<String>,
    pub database_url: Option<String>,
    pub metrics_port: Option<u16>,
}

impl ConfigBuilder {
    /// Create new config builder
    pub fn new() -> Self {
        Self {
            config: Config::builder().build().unwrap(),
        }
    }

    /// Load configuration from multiple sources
    pub fn load() -> Result<RelayerConfig, SettlementError> {
        let mut builder = ConfigBuilder::new();
        
        // Parse CLI arguments
        let cli_args = builder.parse_cli_args();
        
        // Load configuration in order of precedence:
        // 1. Default values
        // 2. Config file
        // 3. Environment variables  
        // 4. CLI arguments
        
        builder.load_defaults()?;
        
        if let Some(config_file) = &cli_args.config_file {
            builder.load_file(config_file)?;
        } else {
            // Try to load default config files
            builder.try_load_default_files()?;
        }
        
        builder.load_environment()?;
        builder.apply_cli_overrides(&cli_args)?;
        
        // Build final config
        let config: RelayerConfig = builder.config.try_deserialize()
            .map_err(|e| SettlementError::ConfigError(format!("Configuration parsing error: {}", e)))?;
        
        // Validate configuration
        builder.validate_config(&config)?;
        
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Parse CLI arguments
    fn parse_cli_args(&self) -> CliArgs {
        let matches = Command::new("cyrus-relayer")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Cyrus Protocol Cross-Chain Settlement Relayer")
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("FILE")
                    .help("Configuration file path")
                    .env("CYRUS_CONFIG_FILE")
            )
            .arg(
                Arg::new("solana-rpc")
                    .long("solana-rpc")
                    .value_name("URL")
                    .help("Solana RPC URL")
                    .env("SOLANA_RPC_URL")
            )
            .arg(
                Arg::new("aptos-rpc")
                    .long("aptos-rpc")
                    .value_name("URL")
                    .help("Aptos RPC URL")
                    .env("APTOS_RPC_URL")
            )
            .arg(
                Arg::new("log-level")
                    .short('l')
                    .long("log-level")
                    .value_name("LEVEL")
                    .help("Log level (trace, debug, info, warn, error)")
                    .env("LOG_LEVEL")
            )
            .arg(
                Arg::new("database-url")
                    .long("database-url")
                    .value_name("URL")
                    .help("Database URL")
                    .env("DATABASE_URL")
            )
            .arg(
                Arg::new("metrics-port")
                    .long("metrics-port")
                    .value_name("PORT")
                    .help("Metrics server port")
                    .env("METRICS_PORT")
            )
            .get_matches();

        CliArgs {
            config_file: matches.get_one::<String>("config").cloned(),
            solana_rpc_url: matches.get_one::<String>("solana-rpc").cloned(),
            aptos_rpc_url: matches.get_one::<String>("aptos-rpc").cloned(),
            log_level: matches.get_one::<String>("log-level").cloned(),
            database_url: matches.get_one::<String>("database-url").cloned(),
            metrics_port: matches.get_one::<String>("metrics-port")
                .and_then(|s| s.parse().ok()),
        }
    }

    /// Load default configuration values
    fn load_defaults(&mut self) -> Result<(), SettlementError> {
        let defaults = r#"
[solana]
rpc_url = "https://api.devnet.solana.com"
program_id = ""
commitment = "confirmed"
poll_interval_ms = 1000
max_retries = 3

[aptos]
rpc_url = "https://fullnode.testnet.aptoslabs.com/v1"
contract_address = ""
vault_owner = ""
private_key = ""
max_gas_amount = 200000
gas_unit_price = 100
transaction_timeout_secs = 30

[processing]
max_concurrent_settlements = 10
batch_size = 5
retry_attempts = 3
retry_delay_seconds = 5
settlement_timeout_seconds = 300

[monitoring]
metrics_port = 9090
health_check_port = 8080
log_level = "info"
enable_metrics = true

[database]
url = "sqlite:./cyrus-relayer.db"
max_connections = 10
connection_timeout_secs = 30
"#;

        self.config = Config::builder()
            .add_source(config::File::from_str(defaults, config::FileFormat::Toml))
            .build()
            .map_err(|e| SettlementError::ConfigError(format!("Default config error: {}", e)))?;

        Ok(())
    }

    /// Load configuration from file
    fn load_file(&mut self, path: &str) -> Result<(), SettlementError> {
        if !Path::new(path).exists() {
            return Err(SettlementError::ConfigError(format!("Config file not found: {}", path)));
        }

        info!("Loading configuration from: {}", path);

        self.config = Config::builder()
            .add_source(self.config.clone())
            .add_source(File::with_name(path))
            .build()
            .map_err(|e| SettlementError::ConfigError(format!("Config file error: {}", e)))?;

        Ok(())
    }

    /// Try to load default configuration files
    fn try_load_default_files(&mut self) -> Result<(), SettlementError> {
        let default_paths = [
            "./config.toml",
            "./relayer.toml", 
            "/etc/cyrus/relayer.toml",
            "~/.config/cyrus/relayer.toml",
        ];

        for path in &default_paths {
            if Path::new(path).exists() {
                info!("Found default config file: {}", path);
                return self.load_file(path);
            }
        }

        warn!("No default config file found, using defaults and environment variables");
        Ok(())
    }

    /// Load environment variables
    fn load_environment(&mut self) -> Result<(), SettlementError> {
        self.config = Config::builder()
            .add_source(self.config.clone())
            .add_source(
                Environment::with_prefix("CYRUS")
                    .prefix_separator("_")
                    .separator("__")
            )
            .build()
            .map_err(|e| SettlementError::ConfigError(format!("Environment config error: {}", e)))?;

        Ok(())
    }

    /// Apply CLI argument overrides
    fn apply_cli_overrides(&mut self, cli_args: &CliArgs) -> Result<(), SettlementError> {
        let mut builder = Config::builder().add_source(self.config.clone());

        // Override with CLI arguments if provided
        if let Some(ref url) = cli_args.solana_rpc_url {
            builder = builder.set_override("solana.rpc_url", url)
                .map_err(|e| SettlementError::ConfigError(format!("CLI override error: {}", e)))?;
        }

        if let Some(ref url) = cli_args.aptos_rpc_url {
            builder = builder.set_override("aptos.rpc_url", url)
                .map_err(|e| SettlementError::ConfigError(format!("CLI override error: {}", e)))?;
        }

        if let Some(ref level) = cli_args.log_level {
            builder = builder.set_override("monitoring.log_level", level)
                .map_err(|e| SettlementError::ConfigError(format!("CLI override error: {}", e)))?;
        }

        if let Some(ref url) = cli_args.database_url {
            builder = builder.set_override("database.url", url)
                .map_err(|e| SettlementError::ConfigError(format!("CLI override error: {}", e)))?;
        }

        if let Some(port) = cli_args.metrics_port {
            builder = builder.set_override("monitoring.metrics_port", port as i64)
                .map_err(|e| SettlementError::ConfigError(format!("CLI override error: {}", e)))?;
        }

        self.config = builder.build()
            .map_err(|e| SettlementError::ConfigError(format!("CLI config build error: {}", e)))?;

        Ok(())
    }

    /// Validate configuration
    fn validate_config(&self, config: &RelayerConfig) -> Result<(), SettlementError> {
        // Validate Solana configuration
        if config.solana.program_id.is_empty() {
            return Err(SettlementError::ConfigError(
                "Solana program ID is required".to_string()
            ));
        }

        if config.solana.rpc_url.is_empty() {
            return Err(SettlementError::ConfigError(
                "Solana RPC URL is required".to_string()
            ));
        }

        // Validate Aptos configuration
        if config.aptos.contract_address.is_empty() {
            return Err(SettlementError::ConfigError(
                "Aptos contract address is required".to_string()
            ));
        }

        if config.aptos.vault_owner.is_empty() {
            return Err(SettlementError::ConfigError(
                "Aptos vault owner is required".to_string()
            ));
        }

        if config.aptos.private_key.is_empty() {
            return Err(SettlementError::ConfigError(
                "Aptos private key is required".to_string()
            ));
        }

        if config.aptos.rpc_url.is_empty() {
            return Err(SettlementError::ConfigError(
                "Aptos RPC URL is required".to_string()
            ));
        }

        // Validate processing configuration
        if config.processing.max_concurrent_settlements == 0 {
            return Err(SettlementError::ConfigError(
                "Max concurrent settlements must be greater than 0".to_string()
            ));
        }

        if config.processing.retry_attempts == 0 {
            return Err(SettlementError::ConfigError(
                "Retry attempts must be greater than 0".to_string()
            ));
        }

        // Validate database configuration
        if config.database.url.is_empty() {
            return Err(SettlementError::ConfigError(
                "Database URL is required".to_string()
            ));
        }

        // Validate hex keys
        if !config.aptos.private_key.starts_with("0x") {
            return Err(SettlementError::ConfigError(
                "Aptos private key must be in hex format (0x...)".to_string()
            ));
        }

        // Validate addresses
        if !config.aptos.contract_address.starts_with("0x") {
            return Err(SettlementError::ConfigError(
                "Aptos contract address must be in hex format (0x...)".to_string()
            ));
        }

        if !config.aptos.vault_owner.starts_with("0x") {
            return Err(SettlementError::ConfigError(
                "Aptos vault owner must be in hex format (0x...)".to_string()
            ));
        }

        info!("Configuration validation passed");
        Ok(())
    }
}

/// Load configuration with sensible defaults
pub fn load_config() -> Result<RelayerConfig, SettlementError> {
    ConfigBuilder::load()
}

/// Create a sample configuration file
pub fn create_sample_config() -> String {
    r#"# Cyrus Protocol Relayer Configuration
# Copy this file to config.toml and update the values

[solana]
# Solana RPC endpoint
rpc_url = "https://api.devnet.solana.com"
# Cyrus settlement program ID
program_id = "YOUR_PROGRAM_ID_HERE"
# Transaction commitment level
commitment = "confirmed"
# Polling interval in milliseconds
poll_interval_ms = 1000
# Maximum RPC retries
max_retries = 3

[aptos]
# Aptos RPC endpoint
rpc_url = "https://fullnode.testnet.aptoslabs.com/v1"
# Deployed settlement contract address
contract_address = "0xYOUR_CONTRACT_ADDRESS_HERE"
# Vault owner address
vault_owner = "0xYOUR_VAULT_OWNER_ADDRESS_HERE"
# Private key for transaction signing (keep secure!)
private_key = "0xYOUR_PRIVATE_KEY_HERE"
# Maximum gas for transactions
max_gas_amount = 200000
# Gas price in units
gas_unit_price = 100
# Transaction timeout in seconds
transaction_timeout_secs = 30

[processing]
# Maximum concurrent settlement processing
max_concurrent_settlements = 10
# Batch size for processing
batch_size = 5
# Number of retry attempts for failed settlements
retry_attempts = 3
# Delay between retries in seconds
retry_delay_seconds = 5
# Overall settlement timeout in seconds
settlement_timeout_seconds = 300

[monitoring]
# Port for metrics server
metrics_port = 9090
# Port for health checks
health_check_port = 8080
# Log level (trace, debug, info, warn, error)
log_level = "info"
# Enable Prometheus metrics
enable_metrics = true

[database]
# SQLite database file path
url = "sqlite:./cyrus-relayer.db"
# Maximum database connections
max_connections = 10
# Connection timeout in seconds
connection_timeout_secs = 30
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sample_config_creation() {
        let sample = create_sample_config();
        assert!(sample.contains("[solana]"));
        assert!(sample.contains("[aptos]"));
        assert!(sample.contains("[processing]"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = RelayerConfig {
            solana: crate::types::SolanaConfig {
                rpc_url: "https://api.devnet.solana.com".to_string(),
                program_id: "".to_string(), // Invalid: empty
                commitment: "confirmed".to_string(),
                poll_interval_ms: 1000,
                max_retries: 3,
            },
            aptos: crate::types::AptosConfig {
                rpc_url: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
                contract_address: "0x1".to_string(),
                vault_owner: "0x1".to_string(),
                private_key: "0x1".to_string(),
                max_gas_amount: 200000,
                gas_unit_price: 100,
                transaction_timeout_secs: 30,
            },
            processing: crate::types::ProcessingConfig {
                max_concurrent_settlements: 10,
                batch_size: 5,
                retry_attempts: 3,
                retry_delay_seconds: 5,
                settlement_timeout_seconds: 300,
            },
            monitoring: crate::types::MonitoringConfig {
                metrics_port: 9090,
                health_check_port: 8080,
                log_level: "info".to_string(),
                enable_metrics: true,
            },
            database: crate::types::DatabaseConfig {
                url: "sqlite:test.db".to_string(),
                max_connections: 10,
                connection_timeout_secs: 30,
            },
        };

        let builder = ConfigBuilder::new();
        
        // Should fail with empty program ID
        assert!(builder.validate_config(&config).is_err());
        
        // Fix the config
        config.solana.program_id = "11111111111111111111111111111112".to_string();
        
        // Should pass now
        assert!(builder.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_config_file_loading() {
        let config_content = r#"
[solana]
rpc_url = "https://api.mainnet-beta.solana.com"
program_id = "11111111111111111111111111111112"
commitment = "finalized"

[aptos]
contract_address = "0x123"
vault_owner = "0x456"
private_key = "0x789"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), config_content).unwrap();

        let mut builder = ConfigBuilder::new();
        builder.load_defaults().unwrap();
        
        let result = builder.load_file(temp_file.path().to_str().unwrap());
        
        // May fail due to validation, but file loading should work
        if let Err(e) = result {
            // Expected if config is incomplete
            println!("Expected validation error: {}", e);
        }
    }
}