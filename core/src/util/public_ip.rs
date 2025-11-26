// check-if-email-exists
// Copyright (C) 2018-2023 Reacher

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const PUBLIC_IP_CACHE_DURATION: Duration = Duration::from_secs(300);
const PUBLIC_IP_SERVICES: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
    "https://ipecho.net/plain",
];

struct CachedPublicIp {
    ip: Option<String>,
    last_fetched: Option<Instant>,
}

static PUBLIC_IP_CACHE: Lazy<RwLock<CachedPublicIp>> = Lazy::new(|| {
    RwLock::new(CachedPublicIp {
        ip: None,
        last_fetched: None,
    })
});

pub async fn get_public_ip() -> String {
    {
        let cache = PUBLIC_IP_CACHE.read().unwrap();
        if let (Some(ip), Some(last_fetched)) = (&cache.ip, cache.last_fetched) {
            if last_fetched.elapsed() < PUBLIC_IP_CACHE_DURATION {
                return ip.clone();
            }
        }
    }

    let ip = fetch_public_ip().await;

    {
        let mut cache = PUBLIC_IP_CACHE.write().unwrap();
        cache.ip = Some(ip.clone());
        cache.last_fetched = Some(Instant::now());
    }

    ip
}

async fn fetch_public_ip() -> String {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    for service_url in PUBLIC_IP_SERVICES {
        match client.get(*service_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(ip) = response.text().await {
                        let ip = ip.trim().to_string();
                        if is_valid_ip(&ip) {
                            return format!("local:{}", ip);
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }

    get_local_hostname()
}

fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

fn get_local_hostname() -> String {
    match hostname::get() {
        Ok(hostname) => format!("local:{}", hostname.to_string_lossy()),
        Err(_) => "local:unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_public_ip() {
        let ip = get_public_ip().await;
        assert!(ip.starts_with("local:"));
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("8.8.8.8"));
        assert!(is_valid_ip("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
        assert!(!is_valid_ip("not-an-ip"));
        assert!(!is_valid_ip(""));
    }
}
