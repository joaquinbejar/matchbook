//! Database module for Matchbook indexer.
//!
//! Provides database connection, migrations, and query utilities.

pub mod models;
pub mod schema;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::path::Path;
use thiserror::Error;

/// Database connection pool and utilities.
pub struct Database {
    pool: PgPool,
}

/// Database errors.
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Connection error.
    #[error("Failed to connect to database: {0}")]
    Connection(#[from] sqlx::Error),

    /// Migration error.
    #[error("Failed to run migrations: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Query error.
    #[error("Query failed: {0}")]
    Query(String),

    /// Not found error.
    #[error("Record not found: {0}")]
    NotFound(String),
}

impl Database {
    /// Creates a new database connection pool.
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL connection URL
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails.
    pub async fn connect(database_url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Runs all pending database migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if any migration fails.
    pub async fn run_migrations(&self) -> Result<(), DatabaseError> {
        let migrator = Migrator::new(Path::new("./migrations")).await?;
        migrator.run(&self.pool).await?;
        Ok(())
    }

    /// Returns a reference to the connection pool.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Checks if the database connection is healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<(), DatabaseError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Connection)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_display() {
        let err = DatabaseError::NotFound("market".to_string());
        assert_eq!(err.to_string(), "Record not found: market");
    }

    #[test]
    fn test_database_error_query() {
        let err = DatabaseError::Query("invalid syntax".to_string());
        assert_eq!(err.to_string(), "Query failed: invalid syntax");
    }
}
