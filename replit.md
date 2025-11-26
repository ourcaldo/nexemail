# Overview

Reacher is an open-source email verification service that checks if email addresses exist without sending actual emails. The project provides both a Rust library/CLI for direct email verification and an HTTP backend server that can be deployed via Docker. The core verification logic performs SMTP checks, DNS lookups, and various heuristics to determine email deliverability with results classified as `safe`, `risky`, `invalid`, or `unknown`.

# User Preferences

Preferred communication style: Simple, everyday language.

# Recent Changes

## Reason Field Enhancement (November 2025)

Added a required `reason` field to all API responses that explains why an email is classified as `safe`, `risky`, `invalid`, or `unknown`. This field provides human-readable explanations for debugging and understanding verification results.

### Reason Field Values:

- **Safe**: `"Email verification passed all checks"`
- **Risky**: Lists specific issues found:
  - `"Risky: disposable email address"`
  - `"Risky: role-based account (e.g., admin@, support@)"`
  - `"Risky: catch-all address (accepts all emails)"`
  - `"Risky: inbox is full"`
  - Multiple issues are combined: `"Risky: disposable email address, role-based account"`
- **Invalid**: Lists specific issues found:
  - `"Invalid: email syntax is invalid"`
  - `"Invalid: no MX records found for domain"`
  - `"Invalid: cannot connect to SMTP server"`
  - `"Invalid: email account is disabled"`
  - `"Invalid: email is not deliverable"`
- **Unknown**: Includes specific error information:
  - `"Unknown: SMTP connection timed out after Xs"`
  - `"Unknown: SOCKS5 proxy connection failed - <error>"`
  - `"Unknown: Yahoo verification failed - <error>"`
  - `"Unknown: Gmail verification failed - <error>"`
  - `"Unknown: Microsoft 365 verification failed - <error>"`
  - `"Unknown: MX lookup failed - <error>"`

### Files Modified:
- `core/src/lib.rs` - Added `calculate_reachable_with_reason()` and `format_smtp_error_reason()` functions
- `core/src/util/input_output.rs` - Added `reason` field to `CheckEmailOutput` struct and serialization

### Example API Response:
```json
{
  "input": "test@example.com",
  "is_reachable": "risky",
  "reason": "Risky: disposable email address",
  "misc": { ... },
  "mx": { ... },
  "smtp": { ... },
  "syntax": { ... },
  "debug": { ... }
}
```

## Proxy Rotation Bug Fix (November 2025)

Fixed critical bug where 981 SOCKS5 proxies failed to rotate properly (always using the same proxy). The root cause was that `ProxyRotator` was being instantiated fresh inside `check_smtp()` for each request, causing the counter to always reset to 0 and preventing true round-robin rotation.

### Root Cause:
The `ProxyRotator` was created on-demand for each email verification request rather than being shared across requests.

### Solution:
1. **Shared ProxyRotator**: Created `Arc<ProxyRotator>` stored in `BackendConfig` with proper serde skip
2. **Initialization at Startup**: Added `init_proxy_rotator()` method in `BackendConfig` that creates the shared rotator when configuration is loaded
3. **HTTP Path Integration**: Updated API endpoint (`post.rs`) to inject shared rotator from config into `CheckEmailInput`
4. **Worker Path Integration**: Updated worker (`do_work.rs`) to inject shared rotator from config into deserialized task input before calling `check_email()`

### Files Modified:
- `core/src/util/input_output.rs` - Added `proxy_rotator: Option<Arc<ProxyRotator>>` field with `#[serde(skip)]`
- `core/src/smtp/proxy_rotator.rs` - Implemented `Debug` trait for `ProxyRotator`
- `core/src/smtp/mod.rs` - Updated `check_smtp()` to use shared rotator from input
- `backend/src/config.rs` - Added shared rotator storage, `init_proxy_rotator()`, and `get_proxy_rotator()` methods
- `backend/src/http/v0/check_email/post.rs` - Injects shared rotator into API requests
- `backend/src/worker/do_work.rs` - Injects shared rotator into worker task processing

## Enhanced SOCKS5 Error Messages (November 2025)

Enhanced SOCKS5 proxy error messages to provide specific, actionable information instead of generic "General failure" messages.

### Before:
```
SOCKS5 proxy error: General failure
```

### After:
```
SOCKS5 General Failure (reply code 0x01): The proxy server encountered an internal error and could not complete the request. Possible causes: (1) The proxy cannot reach the target SMTP server - check if the target is accessible from the proxy's network. (2) The proxy has internal configuration issues or is overloaded. (3) Firewall or security policy on the proxy is blocking this connection. (4) The proxy's outbound network is restricted. Try using a different proxy or verify the target server is reachable from the proxy's location.
```

### Error Codes Covered:
- **0x01 General Failure**: Proxy internal error with troubleshooting steps
- **0x02 Connection Not Allowed**: Proxy ruleset blocks connection
- **0x03 Network Unreachable**: Proxy cannot route to target network
- **0x04 Host Unreachable**: Target SMTP server not responding
- **0x05 Connection Refused**: SMTP server refused connection
- **0x06 TTL Expired**: Connection timeout due to TTL
- **0x07 Command Not Supported**: Proxy doesn't support CONNECT
- **0x08 Address Type Not Supported**: Address format not supported
- **Connection Timeout**: Proxy/target didn't respond in time

### Additional SOCKS5 Error Types:
- `SocksVersionMismatch`: Wrong SOCKS protocol version
- `AuthenticationRequired/Rejected`: Proxy auth issues
- `ExceededMaxDomainLen`: Domain name too long
- `InvalidDomainType/AuthMethod`: Protocol format issues
- Various timeout and connection errors

### Files Modified:
- `core/src/smtp/error.rs` - Added `format_socks5_error_detailed()` and `format_socks5_reply_error()` functions
- `core/src/lib.rs` - Integrated enhanced SOCKS5 descriptions into `format_smtp_error_reason()`

## Proxy Rotation Configuration (November 2025)

Added automatic proxy rotation functionality that allows Reacher to automatically rotate between multiple defined proxies for load balancing.

### Configuration:
- **Enable rotation**: Set `RCH__PROXY_POOL__ENABLED=true` (or in TOML: `[proxy_pool] enabled = true`)
- **Strategy**: Set `RCH__PROXY_POOL__STRATEGY=round_robin` or `random`

### How It Works:
1. Define multiple proxies in `[overrides.proxies]` section
2. Enable proxy pool rotation
3. Requests automatically rotate through all proxies

### Priority Order:
1. Provider-specific routing (e.g., `RCH__OVERRIDES__GMAIL__PROXY=proxy1`) always takes priority
2. If no provider routing, rotation uses next proxy from pool
3. Fallback to default proxy if rotation is disabled

### Files Modified:
- `core/src/smtp/verif_method.rs` - Added `ProxyRotationStrategy`, `ProxyPoolConfig`, `get_proxy_with_rotation()`
- `core/src/smtp/proxy_rotator.rs` - New module for proxy rotation logic
- `core/src/smtp/mod.rs` - Updated `check_smtp()` to use rotator
- `backend/src/config.rs` - Added `proxy_pool` to `BackendConfig`
- `backend/backend_config.toml` - Added `[proxy_pool]` configuration section

### Example Configuration:
```toml
[proxy_pool]
enabled = true
strategy = "round_robin"

[overrides.proxies]
proxy1 = { host = "proxy1.example.com", port = 1080 }
proxy2 = { host = "proxy2.example.com", port = 1080 }
proxy3 = { host = "proxy3.example.com", port = 1080 }
```

Or via environment variables:
```bash
RCH__PROXY_POOL__ENABLED=true
RCH__PROXY_POOL__STRATEGY=round_robin
RCH__OVERRIDES__PROXIES__PROXY1__HOST=proxy1.example.com
RCH__OVERRIDES__PROXIES__PROXY1__PORT=1080
RCH__OVERRIDES__PROXIES__PROXY2__HOST=proxy2.example.com
RCH__OVERRIDES__PROXIES__PROXY2__PORT=1080
```

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