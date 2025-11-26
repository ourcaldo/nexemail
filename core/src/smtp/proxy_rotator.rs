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

use std::sync::atomic::{AtomicUsize, Ordering};

use rand::seq::SliceRandom;

use super::verif_method::ProxyRotationStrategy;

/// A thread-safe proxy rotator that cycles through a list of proxy IDs.
/// Supports round-robin and random selection strategies.
pub struct ProxyRotator {
        proxy_ids: Vec<String>,
        counter: AtomicUsize,
        strategy: ProxyRotationStrategy,
}

impl std::fmt::Debug for ProxyRotator {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("ProxyRotator")
                        .field("proxy_ids_count", &self.proxy_ids.len())
                        .field("counter", &self.counter.load(std::sync::atomic::Ordering::SeqCst))
                        .field("strategy", &self.strategy)
                        .finish()
        }
}

impl ProxyRotator {
        /// Create a new ProxyRotator with the given proxy IDs and rotation strategy.
        pub fn new(proxy_ids: Vec<String>, strategy: ProxyRotationStrategy) -> Self {
                Self {
                        proxy_ids,
                        counter: AtomicUsize::new(0),
                        strategy,
                }
        }

        /// Get the next proxy ID based on the rotation strategy.
        /// Returns None if there are no proxies configured.
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

        /// Get the number of proxies in the pool.
        pub fn len(&self) -> usize {
                self.proxy_ids.len()
        }

        /// Check if the proxy pool is empty.
        pub fn is_empty(&self) -> bool {
                self.proxy_ids.is_empty()
        }
}

#[cfg(test)]
mod tests {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn test_round_robin_rotation() {
                let proxy_ids = vec![
                        "proxy1".to_string(),
                        "proxy2".to_string(),
                        "proxy3".to_string(),
                ];
                let rotator = ProxyRotator::new(proxy_ids.clone(), ProxyRotationStrategy::RoundRobin);

                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy1".to_string()));
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy2".to_string()));
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy3".to_string()));
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy1".to_string()));
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy2".to_string()));
        }

        #[test]
        fn test_random_rotation() {
                let proxy_ids = vec![
                        "proxy1".to_string(),
                        "proxy2".to_string(),
                        "proxy3".to_string(),
                ];
                let rotator = ProxyRotator::new(proxy_ids.clone(), ProxyRotationStrategy::Random);

                let mut seen: HashSet<String> = HashSet::new();
                for _ in 0..100 {
                        if let Some(id) = rotator.get_next_proxy_id() {
                                seen.insert(id.clone());
                        }
                }
                assert!(seen.len() > 1, "Random should eventually select different proxies");
        }

        #[test]
        fn test_empty_proxy_list() {
                let rotator = ProxyRotator::new(vec![], ProxyRotationStrategy::RoundRobin);
                assert!(rotator.get_next_proxy_id().is_none());
                assert!(rotator.is_empty());
        }

        #[test]
        fn test_single_proxy() {
                let rotator = ProxyRotator::new(
                        vec!["proxy1".to_string()],
                        ProxyRotationStrategy::RoundRobin,
                );
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy1".to_string()));
                assert_eq!(rotator.get_next_proxy_id(), Some(&"proxy1".to_string()));
                assert_eq!(rotator.len(), 1);
        }
}
