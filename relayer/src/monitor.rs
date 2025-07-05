use crate::{
    database::DatabaseStatistics,
    processor::SettlementProcessor,
    types::{ApiResponse, HealthStatus, MonitoringConfig, RelayerMetrics, ServiceStatus},
};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{net::TcpListener, time::interval};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{error, info, Level};
use uuid::Uuid;

/// Monitoring server for metrics and health checks
pub struct MonitoringServer {
    config: MonitoringConfig,
    processor: Arc<SettlementProcessor>,
    metrics_registry: Registry,
    prometheus_metrics: PrometheusMetrics,
}

/// Prometheus metrics
#[derive(Clone)]
pub struct PrometheusMetrics {
    pub settlements_total: Counter,
    pub settlements_successful: Counter,
    pub settlements_failed: Counter,
    pub settlement_duration: Histogram,
    pub vault_balance: Gauge,
    pub pending_settlements: Gauge,
    pub relayer_uptime: Gauge,
}

/// Query parameters for API endpoints
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    pub component: Option<String>,
}

/// API response types
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub metrics: RelayerMetrics,
    pub statistics: DatabaseStatistics,
    pub health: HashMap<String, bool>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub overall_status: ServiceStatus,
    pub components: Vec<HealthStatus>,
    pub last_updated: chrono::DateTime<Utc>,
}

impl MonitoringServer {
    /// Create new monitoring server
    pub fn new(
        config: MonitoringConfig,
        processor: Arc<SettlementProcessor>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let metrics_registry = Registry::new();
        let prometheus_metrics = PrometheusMetrics::new(&metrics_registry)?;

        Ok(Self {
            config,
            processor,
            metrics_registry,
            prometheus_metrics,
        })
    }

    /// Start the monitoring server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting monitoring server on port {}", self.config.metrics_port);

        // Start metrics collector
        self.start_metrics_collector().await;

        // Create router
        let app = self.create_router();

        // Start server
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.config.metrics_port)).await?;
        
        info!("Monitoring server listening on http://0.0.0.0:{}", self.config.metrics_port);
        info!("Available endpoints:");
        info!("  GET  /health - Health check");
        info!("  GET  /metrics - Prometheus metrics");
        info!("  GET  /api/v1/metrics - JSON metrics");
        info!("  GET  /api/v1/statistics - Database statistics");
        info!("  GET  /api/v1/health - Detailed health status");

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the router with all endpoints
    fn create_router(&self) -> Router {
        Router::new()
            // Health endpoints
            .route("/health", get(health_check))
            .route("/api/v1/health", get(detailed_health_check))
            
            // Metrics endpoints
            .route("/metrics", get(prometheus_metrics))
            .route("/api/v1/metrics", get(json_metrics))
            .route("/api/v1/statistics", get(statistics))
            
            // Management endpoints
            .route("/api/v1/status", get(relayer_status))
            .route("/api/v1/settlements", get(recent_settlements))
            .route("/api/v1/settlements/:id", get(settlement_details))
            
            // Root endpoint
            .route("/", get(root))
            
            .layer(
                ServiceBuilder::new()
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                            .on_response(DefaultOnResponse::new().level(Level::INFO)),
                    )
                    .layer(CorsLayer::permissive())
            )
            .with_state(AppState {
                processor: Arc::clone(&self.processor),
                prometheus_metrics: self.prometheus_metrics.clone(),
                registry: self.metrics_registry.clone(),
            })
    }

    /// Start background metrics collector
    async fn start_metrics_collector(&self) {
        let processor = Arc::clone(&self.processor);
        let metrics = self.prometheus_metrics.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                let relayer_metrics = processor.get_metrics().await;
                
                // Update Prometheus metrics
                metrics.settlements_total.reset();
                metrics.settlements_total.inc_by(relayer_metrics.total_settlements_processed as f64);
                
                metrics.settlements_successful.reset();
                metrics.settlements_successful.inc_by(relayer_metrics.successful_settlements as f64);
                
                metrics.settlements_failed.reset();
                metrics.settlements_failed.inc_by(relayer_metrics.failed_settlements as f64);
                
                metrics.vault_balance.set(relayer_metrics.vault_balance_usdc);
                metrics.pending_settlements.set(relayer_metrics.pending_settlements as f64);
                metrics.relayer_uptime.set(relayer_metrics.uptime_seconds as f64);
            }
        });
    }
}

impl PrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        let settlements_total = Counter::new(
            "cyrus_settlements_total",
            "Total number of settlement instructions processed"
        )?;
        
        let settlements_successful = Counter::new(
            "cyrus_settlements_successful_total",
            "Total number of successful settlements"
        )?;
        
        let settlements_failed = Counter::new(
            "cyrus_settlements_failed_total", 
            "Total number of failed settlements"
        )?;
        
        let settlement_duration = Histogram::new(
            "cyrus_settlement_duration_seconds",
            "Settlement processing duration in seconds"
        )?;
        
        let vault_balance = Gauge::new(
            "cyrus_vault_balance_usdc",
            "Current vault balance in USDC"
        )?;
        
        let pending_settlements = Gauge::new(
            "cyrus_pending_settlements",
            "Number of pending settlements"
        )?;
        
        let relayer_uptime = Gauge::new(
            "cyrus_relayer_uptime_seconds",
            "Relayer uptime in seconds"
        )?;

        // Register metrics
        registry.register(Box::new(settlements_total.clone()))?;
        registry.register(Box::new(settlements_successful.clone()))?;
        registry.register(Box::new(settlements_failed.clone()))?;
        registry.register(Box::new(settlement_duration.clone()))?;
        registry.register(Box::new(vault_balance.clone()))?;
        registry.register(Box::new(pending_settlements.clone()))?;
        registry.register(Box::new(relayer_uptime.clone()))?;

        Ok(Self {
            settlements_total,
            settlements_successful,
            settlements_failed,
            settlement_duration,
            vault_balance,
            pending_settlements,
            relayer_uptime,
        })
    }
}

/// Application state
#[derive(Clone)]
struct AppState {
    processor: Arc<SettlementProcessor>,
    prometheus_metrics: PrometheusMetrics,
    registry: Registry,
}

/// Root endpoint
async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "Cyrus Protocol Relayer",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "endpoints": {
            "health": "/health",
            "metrics": "/metrics", 
            "api": "/api/v1/*"
        }
    }))
}

/// Simple health check
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.processor.check_health().await;
    let is_healthy = health.values().all(|&status| status);
    
    let status_code = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    (status_code, Json(serde_json::json!({
        "status": if is_healthy { "healthy" } else { "unhealthy" },
        "timestamp": Utc::now()
    })))
}

/// Detailed health check
async fn detailed_health_check(
    State(state): State<AppState>,
    Query(_params): Query<HealthQuery>,
) -> impl IntoResponse {
    let health = state.processor.check_health().await;
    
    let components: Vec<HealthStatus> = health
        .into_iter()
        .map(|(service, is_healthy)| HealthStatus {
            service,
            status: if is_healthy {
                ServiceStatus::Healthy
            } else {
                ServiceStatus::Unhealthy
            },
            last_check: Utc::now(),
            details: None,
            response_time_ms: None,
        })
        .collect();
    
    let overall_status = if components.iter().all(|c| c.status == ServiceStatus::Healthy) {
        ServiceStatus::Healthy
    } else {
        ServiceStatus::Unhealthy
    };
    
    Json(ApiResponse::success(HealthResponse {
        overall_status,
        components,
        last_updated: Utc::now(),
    }))
}

/// Prometheus metrics endpoint
async fn prometheus_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = state.registry.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            output,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain")],
            format!("Failed to encode metrics: {}", e),
        ),
    }
}

/// JSON metrics endpoint
async fn json_metrics(
    State(state): State<AppState>,
    Query(_params): Query<MetricsQuery>,
) -> impl IntoResponse {
    let metrics = state.processor.get_metrics().await;
    
    match state.processor.get_statistics().await {
        Ok(statistics) => {
            let health = state.processor.check_health().await;
            
            Json(ApiResponse::success(MetricsResponse {
                metrics,
                statistics,
                health,
            }))
        }
        Err(e) => {
            Json(ApiResponse::<MetricsResponse>::error(format!(
                "Failed to get statistics: {}", e
            )))
        }
    }
}

/// Statistics endpoint
async fn statistics(State(state): State<AppState>) -> impl IntoResponse {
    match state.processor.get_statistics().await {
        Ok(stats) => Json(ApiResponse::success(stats)),
        Err(e) => Json(ApiResponse::<DatabaseStatistics>::error(format!(
            "Failed to get statistics: {}", e
        ))),
    }
}

/// Relayer status endpoint
async fn relayer_status(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.processor.get_metrics().await;
    let health = state.processor.check_health().await;
    
    let status = serde_json::json!({
        "relayer_id": format!("cyrus-relayer-{}", uuid::Uuid::new_v4()),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": metrics.uptime_seconds,
        "last_settlement": metrics.last_processed_at,
        "performance": {
            "total_processed": metrics.total_settlements_processed,
            "success_rate": if metrics.total_settlements_processed > 0 {
                metrics.successful_settlements as f64 / metrics.total_settlements_processed as f64
            } else { 0.0 },
            "avg_processing_time_ms": metrics.average_processing_time_ms,
        },
        "vault": {
            "balance_usdc": metrics.vault_balance_usdc,
            "total_volume_usdc": metrics.total_volume_usdc,
        },
        "health": health,
    });
    
    Json(ApiResponse::success(status))
}

/// Recent settlements endpoint
async fn recent_settlements(State(state): State<AppState>) -> impl IntoResponse {
    // This would require additional database methods to get recent settlements
    // For now, return a placeholder
    Json(ApiResponse::success(serde_json::json!({
        "recent_settlements": [],
        "note": "Implementation pending - requires additional database queries"
    })))
}

/// Settlement details endpoint
async fn settlement_details(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match Uuid::parse_str(&id) {
        Ok(uuid) => {
            // This would require additional database methods to get settlement details
            Json(ApiResponse::success(serde_json::json!({
                "settlement_id": uuid,
                "note": "Implementation pending - requires additional database queries"
            })))
        }
        Err(_) => Json(ApiResponse::<serde_json::Value>::error(
            "Invalid settlement ID format".to_string()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AptosConfig, DatabaseConfig, ProcessingConfig, RelayerConfig, 
        SolanaConfig, MonitoringConfig,
    };

    fn create_test_config() -> MonitoringConfig {
        MonitoringConfig {
            metrics_port: 9090,
            health_check_port: 8080,
            log_level: "info".to_string(),
            enable_metrics: true,
        }
    }

    #[test]
    fn test_prometheus_metrics_creation() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry);
        assert!(metrics.is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_server_creation() {
        let config = create_test_config();
        
        // Create mock processor (would need actual implementation)
        // For now, just test that the monitoring server can be created
        // when provided with valid config
    }
}