# Overview

Reacher is an open-source email verification service that checks if email addresses exist without sending actual emails. The project provides both a Rust library/CLI for direct email verification and an HTTP backend server that can be deployed via Docker. The core verification logic performs SMTP checks, DNS lookups, and various heuristics to determine email deliverability with results classified as `safe`, `risky`, `invalid`, or `unknown`.

# User Preferences

Preferred communication style: Simple, everyday language.

# Recent Changes

## proxy_data Enhancement (November 2025)

Made `proxy_data` a required field in all API responses. The field now always shows connection information:

- **When using a proxy**: Format is `proxy:host:port` or `proxy:host:port@username:password`
- **When using local connection**: Format is `local:ip_address` (public IPv4) or `local:hostname` as fallback

### Files Modified:
- `core/src/util/public_ip.rs` - New utility module for fetching and caching public IP address
- `core/src/util/mod.rs` - Added public_ip module export
- `core/src/smtp/mod.rs` - Updated all verification method structs to include required proxy_data field
- `core/Cargo.toml` - Added hostname dependency

### Key Changes:
1. `SmtpDebugVerifMethodSmtp.proxy_data` changed from `Option<String>` to `String`
2. Added new structs: `SmtpDebugVerifMethodApi`, `SmtpDebugVerifMethodHeadless`, `SmtpDebugVerifMethodSkipped`
3. `format_proxy_data()` function is now async and always returns a String
4. Public IP is cached for 5 minutes to avoid repeated external API calls

# System Architecture

## Core Email Verification Engine

The heart of the system is a Rust-based email verification library (`check-if-email-exists`) that performs multi-layered checks:

- **SMTP Verification**: Connects directly to mail servers to verify email existence via SMTP protocol without sending actual emails
- **DNS Validation**: Checks MX records and domain validity
- **Heuristic Analysis**: Detects disposable emails, role-based accounts (admin@, support@), and catch-all addresses
- **Provider-Specific Logic**: Custom verification methods for major providers (Gmail, Yahoo, Hotmail) with configurable rules per domain/MX server
- **WebDriver Integration**: Uses Chrome WebDriver for headless verification of certain email providers that require browser-based checks

The verification engine is stateless by design, allowing horizontal scaling without coordination between instances.

## Backend Architecture

The backend serves as both an HTTP API server and a background worker system:

- **HTTP Server**: Built with the Warp web framework, exposing REST endpoints for single and bulk email verification
- **API Versioning**: Maintains `/v0/check_email` (legacy) and `/v1/check_email` (current) endpoints with consistent interfaces
- **Worker Architecture**: Optional RabbitMQ-based queue system for processing bulk verification jobs asynchronously
- **Database Layer**: PostgreSQL for storing bulk job metadata and verification results, with embedded migrations using sqlx

The architecture supports two deployment modes:
1. **Simple HTTP mode**: Direct synchronous verification via REST API
2. **Queue-based mode**: Asynchronous processing with RabbitMQ for high-volume workloads

## Configuration System

Configuration is managed through a TOML-based system with environment variable overrides:

- **Proxy Support**: SOCKS5 proxy configuration for routing SMTP traffic through trusted IPs
- **Multiple Proxy Routing**: Advanced routing rules to send traffic to different proxies based on email provider or MX host
- **SMTP Parameters**: Configurable timeout, hello name, from email, and connection settings
- **Throttling & Concurrency**: Rate limiting and parallel request controls for worker mode
- **Provider Overrides**: Customizable verification strategies per email provider

All configuration can be set via environment variables using the `RCH__` prefix with double underscores for nested values (e.g., `RCH__PROXY__HOST`).

## Database Schema

PostgreSQL stores bulk verification data with the following key tables:

- **v1_bulk_job**: Tracks bulk verification jobs with total record count and timestamps
- **v1_task_result**: Stores individual verification results linked to jobs, including payload, result JSON, and error information
- **Legacy tables** (bulk_jobs, email_results): Maintained for backward compatibility with v0 API

The system uses sqlxmq for embedded migrations and database queue management.

# External Dependencies

## Third-Party Services

- **Proxy4Smtp**: Recommended SOCKS5 proxy service optimized for SMTP verification with maintained IP reputation
- **HaveIBeenPwned**: Optional API integration for checking if emails appear in data breaches
- **Gravatar**: Optional check for associated profile images

## Infrastructure Components

- **PostgreSQL**: Primary data store for bulk verification jobs and results (optional, only needed for bulk verification)
- **RabbitMQ**: Message queue for asynchronous task processing in worker mode (optional)
- **ChromeDriver**: Headless browser automation for provider-specific verifications requiring JavaScript

## Deployment Platform

- **Docker**: Primary distribution method via Docker Hub (`reacherhq/backend`)
- **Commercial License Trial**: Special Docker image with pre-configured proxy and usage limits for evaluation

## Rust Dependencies

- **SMTP**: Custom async-smtp library for email protocol communication
- **TLS/SSL**: aws-lc-rs for cryptographic operations, rustls for TLS connections
- **HTTP Framework**: warp for REST API server
- **Async Runtime**: Tokio for asynchronous I/O operations
- **Serialization**: serde for JSON handling, sqlx for database operations
- **Configuration**: config crate for TOML-based settings management

The system requires outbound port 25 access for SMTP verification, which is commonly blocked by ISPs and cloud providers, necessitating proxy usage in most deployment scenarios.