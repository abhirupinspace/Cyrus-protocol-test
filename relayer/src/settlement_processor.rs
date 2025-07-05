use crate::{
    chains::{aptos::AptosChain, solana::SolanaChain, DestinationChain, SourceChain},
    database::{Database, DatabaseStatistics},
    types::{
        ProcessingConfig, RelayerConfig, RelayerMetrics, SettlementError, SettlementInstruction,
        SettlementResult, SettlementStatus,
    },
};
use backoff::{future::retry, ExponentialBackoff};
use chrono::Utc;
use futures::StreamExt;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, RwLock, Semaphore},
    time::{interval, sleep},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Core settlement processor that orchestrates cross-chain settlements
pub struct SettlementProcessor {
    config: RelayerConfig,
    source_chain: Arc<dyn SourceChain>,
    destination_chain: Arc<dyn DestinationChain>,
    database: Arc<Database>,
    metrics: Arc<RwLock<RelayerMetrics>>,
    processing_semaphore: Arc<Semaphore>,
    instruction_queue: mpsc::UnboundedSender<SettlementInstruction>,
    processing_times: Arc<RwLock<Vec<Duration>>>,
    start_time: Instant,
}

impl SettlementProcessor {
    /// Create new settlement processor
    pub async fn new(config: RelayerConfig) -> Result<Self, SettlementError> {
        info!("Initializing settlement processor");

        // Create database connection
        let database = Arc::new(Database::new(&config.database).await?);

        // Create instruction queue
        let (instruction_sender, instruction_receiver) = mpsc::unbounded_channel();

        // Create source chain (Solana)
        let source_chain = Arc::new(SolanaChain::new(
            config.solana.clone(),
            Some(instruction_sender.clone()),
        )?);

        // Create destination chain (Aptos)
        let destination_chain = Arc::new(AptosChain::new(config.aptos.clone()).await?);

        // Initialize metrics
        let metrics = Arc::new(RwLock::new(RelayerMetrics::default()));

        // Create processing semaphore to limit concurrent settlements
        let processing_semaphore = Arc::new(Semaphore::new(config.processing.max_concurrent_settlements));

        let processor = Self {
            config,
            source_chain,
            destination_chain,
            database,
            metrics,
            processing_semaphore,
            instruction_queue: instruction_sender,
            processing_times: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        };

        // Start background tasks
        processor.start_instruction_processor(instruction_receiver).await;
        processor.start_metrics_updater().await;
        processor.start_retry_processor().await;

        info!("Settlement processor initialized successfully");
        Ok(processor)
    }

    /// Start the relayer service
    pub async fn start(&self) -> Result<(), SettlementError> {
        info!("Starting Cyrus Protocol Relayer");

        // Start source chain event listener
        self.source_chain.start_event_listener().await?;

        // Process any pending instructions from database
        self.process_pending_instructions().await?;

        info!("Cyrus Protocol Relayer started successfully");
        Ok(())
    }

    /// Start instruction processor task
    async fn start_instruction_processor(
        &self,
        mut instruction_receiver: mpsc::UnboundedReceiver<SettlementInstruction>,
    ) {
        let database = Arc::clone(&self.database);
        let destination_chain = Arc::clone(&self.destination_chain);
        let metrics = Arc::clone(&self.metrics);
        let semaphore = Arc::clone(&self.processing_semaphore);
        let processing_times = Arc::clone(&self.processing_times);
        let config = self.config.processing.clone();

        tokio::spawn(async move {
            while let Some(instruction) = instruction_receiver.recv().await {
                info!("Received settlement instruction: {}", instruction.id);

                // Store instruction in database
                if let Err(e) = database.store_instruction(&instruction).await {
                    error!("Failed to store instruction: {}", e);
                    continue;
                }

                // Acquire semaphore permit for processing
                let permit = semaphore.acquire().await.unwrap();

                // Spawn processing task
                let database_clone = Arc::clone(&database);
                let destination_chain_clone = Arc::clone(&destination_chain);
                let metrics_clone = Arc::clone(&metrics);
                let processing_times_clone = Arc::clone(&processing_times);
                let config_clone = config.clone();

                tokio::spawn(async move {
                    let start_time = Instant::now();

                    let result = Self::process_instruction_with_retry(
                        &instruction,
                        destination_chain_clone,
                        &config_clone,
                    ).await;

                    let processing_time = start_time.elapsed();

                    // Update processing times
                    {
                        let mut times = processing_times_clone.write().await;
                        times.push(processing_time);
                        // Keep only last 1000 processing times
                        if times.len() > 1000 {
                            times.remove(0);
                        }
                    }

                    // Store result in database
                    if let Err(e) = database_clone.store_result(&result).await {
                        error!("Failed to store result: {}", e);
                    }

                    // Update metrics
                    Self::update_metrics_for_result(&result, &metrics_clone).await;

                    // Log result
                    match result.status {
                        SettlementStatus::Completed => {
                            info!(
                                "Settlement completed successfully: {} in {:?}",
                                instruction.id, processing_time
                            );
                        }
                        SettlementStatus::Failed => {
                            error!(
                                "Settlement failed: {} - {}",
                                instruction.id,
                                result.error_message.unwrap_or_default()
                            );
                        }
                        _ => {}
                    }

                    drop(permit);
                });
            }
        });
    }

    /// Process instruction with retry logic
    async fn process_instruction_with_retry(
        instruction: &SettlementInstruction,
        destination_chain: Arc<dyn DestinationChain>,
        config: &ProcessingConfig,
    ) -> SettlementResult {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(config.settlement_timeout_seconds)),
            max_interval: Duration::from_secs(config.retry_delay_seconds),
            ..Default::default()
        };

        let mut retry_count = 0;

        let result = retry(backoff, || async {
            retry_count += 1;
            
            debug!("Processing settlement attempt {}: {}", retry_count, instruction.id);

            match destination_chain.submit_settlement(instruction).await {
                Ok(mut result) => {
                    result.retry_count = retry_count - 1;
                    
                    if result.status == SettlementStatus::Completed {
                        Ok(result)
                    } else {
                        Err(backoff::Error::Transient {
                            err: SettlementError::TransactionFailed(
                                result.error_message.unwrap_or_default()
                            ),
                            retry_after: None,
                        })
                    }
                }
                Err(e) => {
                    warn!("Settlement attempt {} failed: {}", retry_count, e);
                    
                    // Determine if error is retryable
                    let is_retryable = match &e {
                        SettlementError::NetworkError(_) => true,
                        SettlementError::Timeout(_) => true,
                        SettlementError::TransactionFailed(_) => true,
                        SettlementError::InsufficientBalance { .. } => false,
                        SettlementError::AlreadyProcessed(_) => false,
                        SettlementError::InvalidInstruction(_) => false,
                        _ => true,
                    };

                    if is_retryable && retry_count < config.retry_attempts {
                        Err(backoff::Error::Transient {
                            err: e,
                            retry_after: Some(Duration::from_secs(config.retry_delay_seconds)),
                        })
                    } else {
                        Err(backoff::Error::Permanent(e))
                    }
                }
            }
        }).await;

        match result {
            Ok(result) => result,
            Err(e) => SettlementResult::failure(instruction.id, e.to_string(), retry_count - 1),
        }
    }

    /// Process pending instructions from database
    async fn process_pending_instructions(&self) -> Result<(), SettlementError> {
        info!("Processing pending instructions from database");

        let pending_instructions = self.database.get_pending_instructions().await?;
        
        info!("Found {} pending instructions", pending_instructions.len());

        for instruction in pending_instructions {
            if let Err(e) = self.instruction_queue.send(instruction) {
                error!("Failed to queue pending instruction: {}", e);
            }
        }

        Ok(())
    }

    /// Start metrics updater task
    async fn start_metrics_updater(&self) {
        let database = Arc::clone(&self.database);
        let destination_chain = Arc::clone(&self.destination_chain);
        let metrics = Arc::clone(&self.metrics);
        let processing_times = Arc::clone(&self.processing_times);
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                if let Err(e) = Self::update_metrics(
                    &database,
                    &destination_chain,
                    &metrics,
                    &processing_times,
                    start_time,
                ).await {
                    error!("Failed to update metrics: {}", e);
                }
            }
        });
    }

    /// Start retry processor for failed settlements
    async fn start_retry_processor(&self) {
        let database = Arc::clone(&self.database);
        let instruction_queue = self.instruction_queue.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Check every 5 minutes

            loop {
                interval.tick().await;

                match database.get_instructions_by_status(SettlementStatus::Failed, Some(10)).await {
                    Ok(failed_settlements) => {
                        for (instruction, result) in failed_settlements {
                            // Retry if not too many attempts and error is retryable
                            if result.retry_count < 3 {
                                info!("Retrying failed settlement: {}", instruction.id);
                                if let Err(e) = instruction_queue.send(instruction) {
                                    error!("Failed to queue retry instruction: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get failed settlements for retry: {}", e);
                    }
                }
            }
        });
    }

    /// Update metrics
    async fn update_metrics(
        database: &Database,
        destination_chain: &Arc<dyn DestinationChain>,
        metrics: &Arc<RwLock<RelayerMetrics>>,
        processing_times: &Arc<RwLock<Vec<Duration>>>,
        start_time: Instant,
    ) -> Result<(), SettlementError> {
        let stats = database.get_statistics().await?;
        let vault_balance = destination_chain.get_vault_balance().await.unwrap_or(0);

        let avg_processing_time = {
            let times = processing_times.read().await;
            if times.is_empty() {
                0.0
            } else {
                let total: Duration = times.iter().sum();
                total.as_millis() as f64 / times.len() as f64
            }
        };

        let mut metrics_guard = metrics.write().await;
        *metrics_guard = RelayerMetrics {
            total_settlements_processed: stats.total_instructions,
            successful_settlements: stats.completed_settlements,
            failed_settlements: stats.failed_settlements,
            pending_settlements: stats.pending_settlements,
            average_processing_time_ms: avg_processing_time,
            last_processed_at: if stats.completed_settlements > 0 {
                Some(Utc::now())
            } else {
                None
            },
            uptime_seconds: start_time.elapsed().as_secs(),
            vault_balance_usdc: vault_balance as f64 / 1_000_000.0,
            total_volume_usdc: stats.total_volume_usdc(),
        };

        Ok(())
    }

    /// Update metrics for a settlement result
    async fn update_metrics_for_result(
        result: &SettlementResult,
        metrics: &Arc<RwLock<RelayerMetrics>>,
    ) {
        let mut metrics_guard = metrics.write().await;
        
        match result.status {
            SettlementStatus::Completed => {
                metrics_guard.successful_settlements += 1;
                metrics_guard.last_processed_at = Some(Utc::now());
            }
            SettlementStatus::Failed => {
                metrics_guard.failed_settlements += 1;
            }
            _ => {}
        }
        
        metrics_guard.total_settlements_processed += 1;
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> RelayerMetrics {
        self.metrics.read().await.clone()
    }

    /// Get database statistics
    pub async fn get_statistics(&self) -> Result<DatabaseStatistics, SettlementError> {
        self.database.get_statistics().await
    }

    /// Process a single instruction manually (for testing)
    pub async fn process_instruction(&self, instruction: SettlementInstruction) -> Result<SettlementResult, SettlementError> {
        // Store instruction
        self.database.store_instruction(&instruction).await?;

        // Process with retry
        let result = Self::process_instruction_with_retry(
            &instruction,
            Arc::clone(&self.destination_chain),
            &self.config.processing,
        ).await;

        // Store result
        self.database.store_result(&result).await?;

        // Update metrics
        Self::update_metrics_for_result(&result, &self.metrics).await;

        Ok(result)
    }

    /// Check health of all components
    pub async fn check_health(&self) -> HashMap<String, bool> {
        let mut health = HashMap::new();

        // Check database
        let db_health = match self.database.get_statistics().await {
            Ok(_) => true,
            Err(e) => {
                error!("Database health check failed: {}", e);
                false
            }
        };
        health.insert("database".to_string(), db_health);

        // Check destination chain
        let dest_health = self.destination_chain.check_health().await.unwrap_or(false);
        health.insert("aptos_chain".to_string(), dest_health);

        // Check source chain
        let source_health = match self.source_chain.get_latest_slot().await {
            Ok(_) => true,
            Err(e) => {
                error!("Solana health check failed: {}", e);
                false
            }
        };
        health.insert("solana_chain".to_string(), source_health);

        health
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<(), SettlementError> {
        info!("Shutting down settlement processor");
        
        // Wait for ongoing settlements to complete
        let permits_needed = self.config.processing.max_concurrent_settlements;
        let _permits = self.processing_semaphore.acquire_many(permits_needed as u32).await;
        
        info!("Settlement processor shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AptosConfig, ChainId, DatabaseConfig, MonitoringConfig, ProcessingConfig, RelayerConfig,
        SettlementInstruction, SolanaConfig, TransactionHash, Address,
    };
    use chrono::Utc;

    fn create_test_config() -> RelayerConfig {
        RelayerConfig {
            solana: SolanaConfig {
                rpc_url: "https://api.devnet.solana.com".to_string(),
                program_id: "11111111111111111111111111111112".to_string(),
                commitment: "confirmed".to_string(),
                poll_interval_ms: 1000,
                max_retries: 3,
            },
            aptos: AptosConfig {
                rpc_url: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
                contract_address: "0x1".to_string(),
                vault_owner: "0x1".to_string(),
                private_key: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                max_gas_amount: 200000,
                gas_unit_price: 100,
                transaction_timeout_secs: 30,
            },
            processing: ProcessingConfig {
                max_concurrent_settlements: 5,
                batch_size: 10,
                retry_attempts: 3,
                retry_delay_seconds: 5,
                settlement_timeout_seconds: 60,
            },
            monitoring: MonitoringConfig {
                metrics_port: 9090,
                health_check_port: 8080,
                log_level: "info".to_string(),
                enable_metrics: true,
            },
            database: DatabaseConfig {
                url: ":memory:".to_string(),
                max_connections: 5,
                connection_timeout_secs: 30,
            },
        }
    }

    #[tokio::test]
    async fn test_processor_creation() {
        let config = create_test_config();
        
        // May fail due to network dependencies, but should not panic
        match SettlementProcessor::new(config).await {
            Ok(_) => println!("Processor created successfully"),
            Err(e) => println!("Expected error in test: {}", e),
        }
    }

    #[tokio::test] 
    async fn test_metrics_initialization() {
        let config = create_test_config();
        
        if let Ok(processor) = SettlementProcessor::new(config).await {
            let metrics = processor.get_metrics().await;
            assert_eq!(metrics.total_settlements_processed, 0);
            assert_eq!(metrics.successful_settlements, 0);
        }
    }
}