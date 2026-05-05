pub mod auth;
pub mod config;
pub mod db;
pub mod email;
pub mod error;
pub mod health;
pub mod metrics_route;
pub mod middleware;
pub mod openapi;
pub mod redis_pool;
pub mod router;
pub mod state;
pub mod telemetry;
pub mod users;

pub use error::{AppError, AppResult};
pub use state::AppState;
