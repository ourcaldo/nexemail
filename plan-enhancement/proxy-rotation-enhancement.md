# Proxy Rotation Enhancement Plan

## 1. Overview

### User Request
Add automatic proxy rotation functionality to Reacher so that when multiple proxies are defined in the configuration (environment variables or TOML file), the system automatically rotates between them for each request - even without explicitly setting routing rules for specific email providers.

### Current Limitation
Currently, if multiple proxies are defined but no routing rules are set:
- Only the "default" proxy is used for all requests
- Other defined proxies sit idle
- No load balancing or rotation occurs

### Desired Behavior
- When multiple proxies are defined, automatically rotate between them
- Support different rotation strategies: round-robin, random, weighted
- Work alongside existing provider-based routing (provider routing takes priority)
- No changes required to API request format

---

## 2. Files Searched / Deep Dived

| File | Purpose |
|------|---------|
| `core/src/smtp/verif_method.rs` | Core verification method definitions, proxy storage (HashMap), `get_proxy()` function |
| `core/src/smtp/connect.rs` | SMTP connection logic, how proxy is used for SOCKS5 connections |
| `core/src/smtp/mod.rs` | Main SMTP check orchestration, calls `get_proxy()` to retrieve proxy |
| `core/src/util/input_output.rs` | `CheckEmailInputProxy` struct definition |
| `backend/src/config.rs` | Backend configuration, `OverridesConfig`, `get_verif_method()` |
| `backend/src/http/v0/check_email/post.rs` | v0 API endpoint, how proxy from request body is handled |
| `backend/src/http/v1/check_email/post.rs` | v1 API endpoint |
| `backend/backend_config.toml` | Configuration file structure |
| `docs/self-hosting/proxies/multiple-proxies.md` | Documentation for current multi-proxy setup |

---

## 3. Current System Understanding

### 3.1 Proxy Storage Architecture

```
VerifMethod struct
├── proxies: HashMap<ProxyID, CheckEmailInputProxy>  ← All proxies stored here
├── gmail: GmailVerifMethod
│   └── Smtp(VerifMethodSmtpConfig)
│       └── proxy: Option<ProxyID>  ← References key in proxies HashMap
├── hotmailb2b: HotmailB2BVerifMethod
├── hotmailb2c: HotmailB2CVerifMethod
├── mimecast: MimecastVerifMethod
├── proofpoint: ProofpointVerifMethod
├── yahoo: YahooVerifMethod
└── everything_else: EverythingElseVerifMethod
    └── Smtp(VerifMethodSmtpConfig)
        └── proxy: Option<ProxyID>  ← This is used for non-specific providers
```

### 3.2 Proxy Configuration Sources

#### Source 1: Environment Variables / TOML (Backend Config)
```rust
// backend/src/config.rs - BackendConfig
pub struct BackendConfig {
    pub proxy: Option<CheckEmailInputProxy>,     // Default proxy (RCH__PROXY__*)
    pub overrides: OverridesConfig,              // Named proxies + routing
}

pub struct OverridesConfig {
    pub proxies: HashMap<String, CheckEmailInputProxy>,  // Named proxies (RCH__OVERRIDES__PROXIES__*)
    pub gmail: Option<GmailVerifMethod>,         // Provider-specific routing
    pub hotmailb2b: Option<HotmailB2BVerifMethod>,
    // ... etc
}
```

#### Source 2: API Request Body
```rust
// backend/src/http/v0/check_email/post.rs
pub struct CheckEmailRequest {
    pub to_email: String,
    pub proxy: Option<CheckEmailInputProxy>,  // Per-request proxy
    // ...
}
```

### 3.3 Proxy Selection Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         API Request Received                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Is `proxy` field present in request body?                               │
│  (backend/src/http/v0/check_email/post.rs:66)                           │
└─────────────────────────────────────────────────────────────────────────┘
                     │                              │
                    YES                            NO
                     │                              │
                     ▼                              ▼
┌────────────────────────────┐    ┌─────────────────────────────────────┐
│ Create VerifMethod with    │    │ Use config.get_verif_method()      │
│ request proxy for ALL      │    │ (Uses backend config proxies)       │
│ providers                  │    │                                     │
│                            │    │ Respects provider routing:          │
│ new_with_same_config_for_  │    │ - Gmail → proxy1                   │
│ all(Some(proxy), ...)      │    │ - Yahoo → proxy2                   │
└────────────────────────────┘    │ - EverythingElse → default proxy   │
                                  └─────────────────────────────────────┘
                                                  │
                                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    check_email() called                                  │
│                    (core/src/lib.rs:202)                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              check_smtp() called                                         │
│              (core/src/smtp/mod.rs:134)                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Determine EmailProvider from MX host                                    │
│  EmailProvider::from_mx_host(&host_str)                                 │
│  (core/src/smtp/verif_method.rs:50-66)                                  │
│                                                                          │
│  Possible values:                                                        │
│  - Gmail, HotmailB2B, HotmailB2C, Yahoo, Mimecast, Proofpoint           │
│  - EverythingElse (fallback)                                            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Get SMTP config for this provider                                       │
│  e.g., input.verif_method.gmail → GmailVerifMethod::Smtp(config)        │
│  (core/src/smtp/mod.rs:146-216)                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Get proxy for this provider                                             │
│  input.verif_method.get_proxy(email_provider)                           │
│  (core/src/smtp/mod.rs:219)                                             │
│                                                                          │
│  THIS IS THE KEY FUNCTION - Currently returns STATIC proxy:             │
│  - Looks up config.proxy (Option<ProxyID>)                              │
│  - Returns self.proxies.get(proxy_id) - ALWAYS SAME PROXY!              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Create VerifMethodSmtp with proxy                                       │
│  VerifMethodSmtp::new(smtp_config, proxy.cloned())                      │
│  (core/src/smtp/mod.rs:221-224)                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Connect via proxy (if present)                                          │
│  connect_to_smtp_host() in core/src/smtp/connect.rs:61                  │
│                                                                          │
│  If proxy exists:                                                        │
│    Socks5Stream::connect(proxy.host, proxy.port, target_host, port)     │
│  Else:                                                                   │
│    TcpStream::connect(target_host, port)                                │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.4 Key Function: `get_proxy()`

```rust
// core/src/smtp/verif_method.rs:224-271
pub fn get_proxy(&self, email_provider: EmailProvider) -> Option<&CheckEmailInputProxy> {
    match email_provider {
        EmailProvider::Gmail => match &self.gmail {
            GmailVerifMethod::Smtp(c) => c
                .proxy                              // Option<ProxyID> - e.g., "proxy1"
                .as_ref()
                .and_then(|proxy_id| self.proxies.get(proxy_id)),  // HashMap lookup
        },
        // ... same pattern for all providers ...
        EmailProvider::EverythingElse => match &self.everything_else {
            EverythingElseVerifMethod::Smtp(c) => c
                .proxy
                .as_ref()
                .and_then(|proxy_id| self.proxies.get(proxy_id)),
        },
    }
}
```

**Problem**: This function always returns the same proxy for a given provider because:
1. The `proxy` field in `VerifMethodSmtpConfig` is static (set at config load time)
2. There's no rotation logic - it's a simple HashMap lookup

### 3.5 CheckEmailInputProxy Structure

```rust
// core/src/util/input_output.rs:96-107
pub struct CheckEmailInputProxy {
    pub host: String,           // SOCKS5 proxy host
    pub port: u16,              // SOCKS5 proxy port
    pub username: Option<String>, // Optional auth
    pub password: Option<String>, // Optional auth
    pub timeout_ms: Option<u64>,  // Connection timeout
}
```

### 3.6 SOCKS5 Connection in connect.rs

```rust
// core/src/smtp/connect.rs:75-110
let stream: BufStream<Box<dyn AsyncReadWrite>> = match &verif_method.proxy {
    Some(proxy) => {
        // Connect through SOCKS5 proxy
        let socks_stream = if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
            Socks5Stream::connect_with_password(
                (proxy.host.as_ref(), proxy.port),
                clean_host.clone(),
                verif_method.config.smtp_port,
                username.clone(),
                password.clone(),
                config,
            ).await?
        } else {
            Socks5Stream::connect(
                (proxy.host.as_ref(), proxy.port),
                clean_host.clone(),
                verif_method.config.smtp_port,
                config,
            ).await?
        };
        BufStream::new(Box::new(socks_stream) as Box<dyn AsyncReadWrite>)
    }
    None => {
        // Direct TCP connection (no proxy)
        let tcp_stream = TcpStream::connect(...).await?;
        BufStream::new(Box::new(tcp_stream) as Box<dyn AsyncReadWrite>)
    }
};
```

---

## 4. Implementation Plan

### Phase 1: Add Proxy Pool Configuration

#### 4.1 Create New Configuration Structure

**File: `core/src/smtp/verif_method.rs`**

Add new types for proxy rotation:

```rust
/// Rotation strategy for proxy pool
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProxyRotationStrategy {
    #[default]
    RoundRobin,      // Cycle through proxies in order
    Random,          // Random selection
    // Future: Weighted, LeastUsed, etc.
}

/// Configuration for proxy pool with rotation
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProxyPoolConfig {
    /// Whether to enable proxy rotation
    pub enabled: bool,
    /// Rotation strategy
    pub strategy: ProxyRotationStrategy,
}
```

#### 4.2 Add Proxy Pool to VerifMethod

**File: `core/src/smtp/verif_method.rs`**

Extend `VerifMethod` struct:

```rust
pub struct VerifMethod {
    pub proxies: HashMap<ProxyID, CheckEmailInputProxy>,
    pub proxy_pool: ProxyPoolConfig,  // NEW: Proxy pool configuration
    // ... existing fields
}
```

### Phase 2: Implement Proxy Rotator

#### 4.3 Create Proxy Rotator Module

**New File: `core/src/smtp/proxy_rotator.rs`**

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use rand::seq::SliceRandom;

pub struct ProxyRotator {
    proxy_ids: Vec<String>,
    counter: AtomicUsize,
    strategy: ProxyRotationStrategy,
}

impl ProxyRotator {
    pub fn new(proxy_ids: Vec<String>, strategy: ProxyRotationStrategy) -> Self {
        Self {
            proxy_ids,
            counter: AtomicUsize::new(0),
            strategy,
        }
    }
    
    pub fn get_next_proxy_id(&self) -> Option<&String> {
        if self.proxy_ids.is_empty() {
            return None;
        }
        
        match self.strategy {
            ProxyRotationStrategy::RoundRobin => {
                let index = self.counter.fetch_add(1, Ordering::SeqCst) % self.proxy_ids.len();
                self.proxy_ids.get(index)
            }
            ProxyRotationStrategy::Random => {
                self.proxy_ids.choose(&mut rand::thread_rng())
            }
        }
    }
}
```

### Phase 3: Modify Proxy Selection Logic

#### 4.4 Update get_proxy() Function

**File: `core/src/smtp/verif_method.rs`**

Modify `get_proxy()` to support rotation:

```rust
/// Get the proxy to use for the email provider.
/// If proxy rotation is enabled and no provider-specific proxy is set,
/// rotate through available proxies.
pub fn get_proxy_with_rotation(
    &self, 
    email_provider: EmailProvider,
    rotator: Option<&ProxyRotator>
) -> Option<&CheckEmailInputProxy> {
    // First, try provider-specific proxy (existing logic)
    let provider_proxy = self.get_provider_specific_proxy(&email_provider);
    
    if provider_proxy.is_some() {
        return provider_proxy;
    }
    
    // If no provider-specific proxy, use rotation if enabled
    if self.proxy_pool.enabled {
        if let Some(rotator) = rotator {
            if let Some(proxy_id) = rotator.get_next_proxy_id() {
                return self.proxies.get(proxy_id);
            }
        }
    }
    
    // Fallback to default proxy
    self.proxies.get(DEFAULT_PROXY_ID)
}
```

### Phase 4: Backend Configuration

#### 4.5 Add Environment Variables

**File: `backend/backend_config.toml`**

Add new configuration section:

```toml
# Proxy pool configuration for automatic rotation
[proxy_pool]
# Enable proxy rotation when multiple proxies are defined
# Env variable: RCH__PROXY_POOL__ENABLED
enabled = false

# Rotation strategy: "round_robin" or "random"
# Env variable: RCH__PROXY_POOL__STRATEGY  
strategy = "round_robin"
```

#### 4.6 Update BackendConfig

**File: `backend/src/config.rs`**

```rust
pub struct BackendConfig {
    // ... existing fields
    pub proxy_pool: ProxyPoolConfig,  // NEW
}
```

### Phase 5: Integration Points

#### 5.1 Update check_smtp() Function

**File: `core/src/smtp/mod.rs`**

```rust
pub async fn check_smtp(
    to_email: &EmailAddress,
    host: &Name,
    domain: &str,
    input: &CheckEmailInput,
) -> (Result<SmtpDetails, SmtpError>, SmtpDebug) {
    // Create rotator if rotation is enabled
    let rotator = if input.verif_method.proxy_pool.enabled {
        let proxy_ids: Vec<String> = input.verif_method.proxies.keys().cloned().collect();
        Some(ProxyRotator::new(proxy_ids, input.verif_method.proxy_pool.strategy.clone()))
    } else {
        None
    };
    
    // Get proxy with rotation support
    let proxy = input.verif_method.get_proxy_with_rotation(email_provider, rotator.as_ref());
    
    // ... rest of function
}
```

### Phase 6: Thread-Safe Global Rotator (Optional Enhancement)

For better consistency across concurrent requests, implement a global rotator:

**File: `backend/src/lib.rs` or new module**

```rust
use once_cell::sync::Lazy;
use std::sync::Arc;

pub static PROXY_ROTATOR: Lazy<Arc<RwLock<Option<ProxyRotator>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

// Initialize during backend startup
pub fn init_proxy_rotator(config: &BackendConfig) {
    if config.proxy_pool.enabled {
        let proxy_ids: Vec<String> = config.get_verif_method().proxies.keys().cloned().collect();
        let rotator = ProxyRotator::new(proxy_ids, config.proxy_pool.strategy.clone());
        *PROXY_ROTATOR.write().unwrap() = Some(rotator);
    }
}
```

---

## 5. File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `core/src/smtp/verif_method.rs` | Modify | Add `ProxyRotationStrategy`, `ProxyPoolConfig`, update `VerifMethod` |
| `core/src/smtp/proxy_rotator.rs` | Create | New module for proxy rotation logic |
| `core/src/smtp/mod.rs` | Modify | Update `check_smtp()` to use rotator |
| `backend/src/config.rs` | Modify | Add `proxy_pool` to `BackendConfig` |
| `backend/backend_config.toml` | Modify | Add `[proxy_pool]` section |
| `docs/self-hosting/proxies/proxy-rotation.md` | Create | Documentation for new feature |

---

## 6. Configuration Examples

### Example 1: Enable Round-Robin Rotation

```bash
# docker-compose.yml
environment:
  # Define multiple proxies
  RCH__OVERRIDES__PROXIES__PROXY1__HOST: proxy1.example.com
  RCH__OVERRIDES__PROXIES__PROXY1__PORT: 1080
  RCH__OVERRIDES__PROXIES__PROXY2__HOST: proxy2.example.com
  RCH__OVERRIDES__PROXIES__PROXY2__PORT: 1080
  RCH__OVERRIDES__PROXIES__PROXY3__HOST: proxy3.example.com
  RCH__OVERRIDES__PROXIES__PROXY3__PORT: 1080
  
  # Enable rotation (no routing rules needed!)
  RCH__PROXY_POOL__ENABLED: true
  RCH__PROXY_POOL__STRATEGY: round_robin
```

### Example 2: Hybrid - Rotation + Provider Routing

```bash
environment:
  # Define proxies
  RCH__OVERRIDES__PROXIES__PROXY1__HOST: proxy1.example.com
  RCH__OVERRIDES__PROXIES__PROXY1__PORT: 1080
  RCH__OVERRIDES__PROXIES__PROXY2__HOST: proxy2.example.com
  RCH__OVERRIDES__PROXIES__PROXY2__PORT: 1080
  
  # Gmail always uses proxy1 (provider routing takes priority)
  RCH__OVERRIDES__GMAIL__TYPE: smtp
  RCH__OVERRIDES__GMAIL__PROXY: proxy1
  
  # Everything else rotates between proxy1 and proxy2
  RCH__PROXY_POOL__ENABLED: true
  RCH__PROXY_POOL__STRATEGY: round_robin
```

---

## 7. Testing Plan

### Unit Tests
1. Test `ProxyRotator` round-robin sequence
2. Test `ProxyRotator` random selection (verify all proxies are eventually selected)
3. Test `get_proxy_with_rotation()` priority (provider > rotation > default)

### Integration Tests
1. Send 10 requests, verify different proxies are used (check `debug.smtp.proxy_data`)
2. Verify provider-specific routing still works when rotation is enabled
3. Verify fallback to default proxy when rotation is disabled

### Load Tests
1. Concurrent requests should distribute across proxies
2. No race conditions in counter increment (atomic operations)

---

## 8. Backward Compatibility

- **No breaking changes**: Rotation is opt-in via `RCH__PROXY_POOL__ENABLED=true`
- **Default behavior unchanged**: If `proxy_pool.enabled = false` (default), existing behavior preserved
- **API unchanged**: No changes to request/response format
- **Provider routing preserved**: Provider-specific proxy settings take priority over rotation
