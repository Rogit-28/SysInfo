//! API route definitions.
//!
//! Defines all REST API endpoints and their corresponding handlers.

use super::handlers;
use super::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

/// Root/index endpoint handler - returns API info and available endpoints
async fn api_index() -> Json<serde_json::Value> {
    Json(json!({
        "name": "SysInfo Telemetry API",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "endpoints": {
            "health": "/api/v1/health",
            "system": "/api/v1/system",
            "cpu": "/api/v1/cpu",
            "gpu": "/api/v1/gpu",
            "memory": "/api/v1/memory",
            "storage": "/api/v1/storage",
            "network": "/api/v1/network",
            "processes": "/api/v1/processes",
            "services": "/api/v1/services",
            "events": "/api/v1/events",
            "updates": "/api/v1/updates",
            "sessions": "/api/v1/sessions"
        }
    }))
}

/// Constant-time string comparison to prevent timing attacks.
/// Returns true if both strings are equal.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let result = a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));
    
    result == 0
}

/// API key authentication middleware.
/// If api_key is configured, validates the X-API-Key header.
/// Uses constant-time comparison to prevent timing attacks.
/// Allows health endpoint to bypass authentication for monitoring purposes.
async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = &state.config.server.api_key;

    // If no API key is configured, allow all requests
    if api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    // Allow health endpoint without authentication (for load balancers, monitoring)
    let path = request.uri().path();
    if path == "/api/v1/health" || path == "/health" {
        return Ok(next.run(request).await);
    }

    // Check for X-API-Key header
    let provided_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match provided_key {
        Some(key) if constant_time_eq(key, api_key) => Ok(next.run(request).await),
        Some(_) => {
            warn!("Invalid API key provided");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            warn!("Missing X-API-Key header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Build CORS layer based on configuration.
fn build_cors_layer(state: &AppState) -> CorsLayer {
    let cors_config = &state.config.server.cors;
    
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-api-key"),
        ])
        .max_age(Duration::from_secs(cors_config.max_age_seconds));

    // Configure allowed origins
    if cors_config.allowed_origins.is_empty() {
        // Empty list = allow all origins (backward compatible default)
        info!("CORS: Allowing all origins (no restrictions configured)");
        cors = cors.allow_origin(Any);
    } else if cors_config.allowed_origins.len() == 1 && cors_config.allowed_origins[0] == "*" {
        // Explicit wildcard
        info!("CORS: Allowing all origins (wildcard configured)");
        cors = cors.allow_origin(Any);
    } else {
        // Specific origins configured
        info!("CORS: Restricting to {} configured origins", cors_config.allowed_origins.len());
        let origins: Vec<_> = cors_config.allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }

    // Configure credentials
    if cors_config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    cors
}

/// Create the main API router with all routes and middleware.
pub fn create_router(state: AppState) -> Router {
    // Configure CORS middleware based on settings
    let cors = build_cors_layer(&state);

    info!("Configuring API routes");

    // Check if auth is enabled
    let auth_enabled = !state.config.server.api_key.is_empty();
    if auth_enabled {
        info!("API key authentication enabled");
    } else {
        info!("API key authentication disabled (no api_key configured)");
    }

    // Build the API v1 router with all endpoints
    let api_v1 = Router::new()
        // Index/root for /api/v1
        .route("/", get(api_index))
        // Health check (no auth required)
        .route("/health", get(handlers::health::get_health))
        // Full system snapshot
        .route("/system", get(handlers::system::get_system))
        // Individual metrics endpoints
        .route("/cpu", get(handlers::cpu::get_cpu))
        .route("/gpu", get(handlers::gpu::get_gpu))
        .route("/memory", get(handlers::memory::get_memory))
        .route("/storage", get(handlers::storage::get_storage))
        .route("/network", get(handlers::network::get_network))
        .route("/processes", get(handlers::processes::get_processes))
        .route("/services", get(handlers::services::get_services))
        .route("/events", get(handlers::events::get_events))
        .route("/updates", get(handlers::updates::get_updates))
        .route("/sessions", get(handlers::sessions::get_sessions));

    // Build the main router with optional auth middleware
    let router = Router::new()
        // Root endpoint shows API info
        .route("/", get(api_index))
        // Nest all API v1 routes under /api/v1
        .nest("/api/v1", api_v1);

    // Apply auth middleware if API key is configured
    let router = if auth_enabled {
        router.layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
    } else {
        router
    };

    router
        // Apply CORS middleware
        .layer(cors)
        // Attach shared state
        .with_state(state)
}
