//! HTTP client implementation.
//!
//! Provides the main HTTP client for interacting with the Matchbook REST API.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Deserialize};

use super::config::ClientConfig;
use super::error::ClientError;
use crate::types::{Balance, Market, Order, OrderBook, Trade};

/// API error response format.
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiError,
}

/// API error details.
#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    message: String,
}

/// Markets list response.
#[derive(Debug, Deserialize)]
struct MarketsResponse {
    markets: Vec<Market>,
}

/// Single market response.
#[derive(Debug, Deserialize)]
struct MarketResponse {
    market: Market,
}

/// Trades list response.
#[derive(Debug, Deserialize)]
struct TradesResponse {
    trades: Vec<Trade>,
}

/// Orders list response.
#[derive(Debug, Deserialize)]
struct OrdersResponse {
    orders: Vec<Order>,
}

/// Balances list response.
#[derive(Debug, Deserialize)]
struct BalancesResponse {
    balances: Vec<Balance>,
}

/// HTTP client for the Matchbook REST API.
#[derive(Debug, Clone)]
pub struct MatchbookClient {
    config: ClientConfig,
    http: reqwest::Client,
}

impl MatchbookClient {
    /// Creates a new client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid or the HTTP client
    /// cannot be created.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        config.validate()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(ref api_key) = config.api_key {
            if let Ok(value) = HeaderValue::from_str(api_key) {
                headers.insert("X-API-Key", value);
            }
        }

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .user_agent(&config.user_agent)
            .build()
            .map_err(ClientError::Request)?;

        Ok(Self { config, http })
    }

    /// Creates a new client with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn with_defaults() -> Result<Self, ClientError> {
        Self::new(ClientConfig::default())
    }

    /// Creates a new client with the given base URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, ClientError> {
        Self::new(ClientConfig::new(base_url))
    }

    /// Returns the client configuration.
    #[must_use]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Makes a GET request to the given path.
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = format!("{}{}", self.config.base_url, path);
        self.request_with_retry(|| self.http.get(&url)).await
    }

    /// Makes a request with retry logic.
    async fn request_with_retry<T, F>(&self, request_fn: F) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut last_error = None;
        let mut retry_count = 0;

        while retry_count <= self.config.max_retries {
            let response = request_fn().send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        let body = resp
                            .text()
                            .await
                            .map_err(|e| ClientError::Deserialization(e.to_string()))?;

                        return serde_json::from_str(&body)
                            .map_err(|e| ClientError::Deserialization(e.to_string()));
                    }

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = resp
                            .headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse().ok());

                        if retry_count < self.config.max_retries {
                            let wait_time = retry_after.unwrap_or(1);
                            tokio::time::sleep(Duration::from_secs(wait_time)).await;
                            retry_count += 1;
                            continue;
                        }

                        return Err(ClientError::RateLimited { retry_after });
                    }

                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Err(ClientError::NotFound("resource".to_string()));
                    }

                    if status == reqwest::StatusCode::UNAUTHORIZED {
                        return Err(ClientError::Unauthorized);
                    }

                    let body = resp.text().await.unwrap_or_default();
                    if let Ok(error_resp) = serde_json::from_str::<ApiErrorResponse>(&body) {
                        return Err(ClientError::Api {
                            code: error_resp.error.code,
                            message: error_resp.error.message,
                        });
                    }

                    return Err(ClientError::Api {
                        code: status.as_str().to_string(),
                        message: body,
                    });
                }
                Err(e) => {
                    if e.is_timeout() && retry_count < self.config.max_retries {
                        retry_count += 1;
                        tokio::time::sleep(Duration::from_millis(100 * (1 << retry_count))).await;
                        last_error = Some(ClientError::from(e));
                        continue;
                    }
                    return Err(ClientError::from(e));
                }
            }
        }

        Err(last_error.unwrap_or(ClientError::Timeout))
    }

    /// Gets all available markets.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_markets(&self) -> Result<Vec<Market>, ClientError> {
        let response: MarketsResponse = self.get("/markets").await?;
        Ok(response.markets)
    }

    /// Gets a specific market by address.
    ///
    /// # Arguments
    ///
    /// * `address` - The market address (base58 encoded)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the market is not found.
    pub async fn get_market(&self, address: &str) -> Result<Market, ClientError> {
        let response: MarketResponse = self.get(&format!("/markets/{}", address)).await?;
        Ok(response.market)
    }

    /// Gets the order book for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - The market address (base58 encoded)
    /// * `depth` - Optional number of price levels (default: 20, max: 500)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_orderbook(
        &self,
        market: &str,
        depth: Option<u32>,
    ) -> Result<OrderBook, ClientError> {
        let path = match depth {
            Some(d) => format!("/markets/{}/orderbook?depth={}", market, d),
            None => format!("/markets/{}/orderbook", market),
        };
        self.get(&path).await
    }

    /// Gets recent trades for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - The market address (base58 encoded)
    /// * `limit` - Optional number of trades (default: 100, max: 1000)
    /// * `before` - Optional cursor for pagination
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_trades(
        &self,
        market: &str,
        limit: Option<u32>,
        before: Option<&str>,
    ) -> Result<Vec<Trade>, ClientError> {
        let mut path = format!("/markets/{}/trades", market);
        let mut params = Vec::new();

        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(b) = before {
            params.push(format!("before={}", b));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response: TradesResponse = self.get(&path).await?;
        Ok(response.trades)
    }

    /// Gets a user's open orders.
    ///
    /// # Arguments
    ///
    /// * `owner` - The wallet address (base58 encoded)
    /// * `market` - Optional market address to filter by
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_orders(
        &self,
        owner: &str,
        market: Option<&str>,
    ) -> Result<Vec<Order>, ClientError> {
        let path = match market {
            Some(m) => format!("/accounts/{}/orders?market={}", owner, m),
            None => format!("/accounts/{}/orders", owner),
        };
        let response: OrdersResponse = self.get(&path).await?;
        Ok(response.orders)
    }

    /// Gets a user's trade history.
    ///
    /// # Arguments
    ///
    /// * `owner` - The wallet address (base58 encoded)
    /// * `market` - Optional market address to filter by
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_user_trades(
        &self,
        owner: &str,
        market: Option<&str>,
    ) -> Result<Vec<Trade>, ClientError> {
        let path = match market {
            Some(m) => format!("/accounts/{}/trades?market={}", owner, m),
            None => format!("/accounts/{}/trades", owner),
        };
        let response: TradesResponse = self.get(&path).await?;
        Ok(response.trades)
    }

    /// Gets a user's balances.
    ///
    /// # Arguments
    ///
    /// * `owner` - The wallet address (base58 encoded)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn get_balances(&self, owner: &str) -> Result<Vec<Balance>, ClientError> {
        let response: BalancesResponse = self.get(&format!("/accounts/{}/balances", owner)).await?;
        Ok(response.balances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let config = ClientConfig::new("https://api.example.com");
        let client = MatchbookClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_defaults() {
        let client = MatchbookClient::with_defaults();
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_base_url() {
        let client = MatchbookClient::with_base_url("https://api.example.com");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_invalid_config() {
        let config = ClientConfig::new("");
        let client = MatchbookClient::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_client_config_access() {
        let config = ClientConfig::new("https://api.example.com").with_api_key("test-key");
        let client = MatchbookClient::new(config).expect("client creation");
        assert_eq!(client.config().base_url, "https://api.example.com");
        assert_eq!(client.config().api_key, Some("test-key".to_string()));
    }
}
