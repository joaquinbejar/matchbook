# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive developer documentation
- OpenAPI 3.0 specification for REST API
- Architecture documentation with diagrams
- Getting started guide for integrators
- API and WebSocket reference documentation
- SDK usage guide for Rust and TypeScript
- FAQ documentation
- Runbooks for operational troubleshooting
- Incident response and maintenance procedures
- Alerting rules with Prometheus Alertmanager
- Monitoring setup with Prometheus and Grafana
- CI/CD pipelines for automated testing and deployment
- Kubernetes manifests for production deployment
- Docker configuration for all services

### Changed
- Updated README with project overview and quick start guide

## [0.1.0] - 2026-01-30

### Added
- Initial release of Matchbook
- On-chain Solana program with order book implementation
- Core instructions: CreateMarket, CreateOpenOrders, Deposit, PlaceOrder, CancelOrder, MatchOrders, ConsumeEvents, Withdraw
- Red-black tree order book data structure
- Event queue for matching events
- Indexer service with Geyser integration
- REST API server with market data endpoints
- WebSocket API for real-time streaming
- Crank service for automated order matching
- Rust SDK for client integration
- TypeScript SDK for web applications
- Docker Compose for local development

### Security
- Non-custodial design with user-controlled funds
- On-chain validation for all operations
- Signed request authentication for trading endpoints

[Unreleased]: https://github.com/joaquinbejar/matchbook/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/joaquinbejar/matchbook/releases/tag/v0.1.0
