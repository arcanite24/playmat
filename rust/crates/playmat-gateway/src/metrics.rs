//! Counters for `/metrics` (Prometheus text format) and the shutdown log.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct Metrics {
    pub connections_open: AtomicU64,
    pub connections_total: AtomicU64,
    pub upstream_failures: AtomicU64,
    pub requests_java: AtomicU64,
    pub requests_rust: AtomicU64,
    pub requests_shadow: AtomicU64,
    pub shadow_same: AtomicU64,
    pub shadow_different: AtomicU64,
    /// calls Rust declined (left to Java), on a rust or shadow route
    pub declined: AtomicU64,
    /// requests refused over the per-connection rate limit
    pub rate_limited: AtomicU64,
    pub frames_to_client: AtomicU64,
    pub bytes_to_client: AtomicU64,
    pub frames_from_client: AtomicU64,
    pub state_patches: AtomicU64,
    pub state_complete: AtomicU64,
    /// what clients would have received without patches
    pub state_full_bytes: AtomicU64,
    pub state_sent_bytes: AtomicU64,
    pub state_resyncs: AtomicU64,
    pub slow_clients_dropped: AtomicU64,
    /// shadow comparisons by method: (same, different); a group moves to `rust` after a week with no differences
    pub shadow_by_method: Mutex<BTreeMap<String, (u64, u64)>>,
    /// declined calls by method (only methods with a Rust handler can be declined, so the list stays short)
    pub declined_by_method: Mutex<BTreeMap<String, u64>>,
}

impl Metrics {
    pub fn add(counter: &AtomicU64, n: u64) {
        counter.fetch_add(n, Ordering::Relaxed);
    }

    pub fn shadow(&self, method: &str, same: bool) {
        Metrics::add(if same { &self.shadow_same } else { &self.shadow_different }, 1);
        let mut by_method = self.shadow_by_method.lock().unwrap();
        let counts = by_method.entry(method.to_string()).or_default();
        if same {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }

    pub fn declined(&self, method: &str) {
        Metrics::add(&self.declined, 1);
        *self.declined_by_method.lock().unwrap().entry(method.to_string()).or_default() += 1;
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut line = |name: &str, kind: &str, value: &AtomicU64| {
            let _ = writeln!(out, "# TYPE playmat_gateway_{name} {kind}");
            let _ = writeln!(out, "playmat_gateway_{name} {}", value.load(Ordering::Relaxed));
        };
        line("connections_open", "gauge", &self.connections_open);
        line("connections_total", "counter", &self.connections_total);
        line("upstream_failures_total", "counter", &self.upstream_failures);
        line("requests_java_total", "counter", &self.requests_java);
        line("requests_rust_total", "counter", &self.requests_rust);
        line("requests_shadow_total", "counter", &self.requests_shadow);
        line("shadow_same_total", "counter", &self.shadow_same);
        line("shadow_different_total", "counter", &self.shadow_different);
        line("declined_total", "counter", &self.declined);
        line("rate_limited_total", "counter", &self.rate_limited);
        line("frames_to_client_total", "counter", &self.frames_to_client);
        line("bytes_to_client_total", "counter", &self.bytes_to_client);
        line("frames_from_client_total", "counter", &self.frames_from_client);
        line("state_patches_total", "counter", &self.state_patches);
        line("state_complete_total", "counter", &self.state_complete);
        line("state_full_bytes_total", "counter", &self.state_full_bytes);
        line("state_sent_bytes_total", "counter", &self.state_sent_bytes);
        line("state_resyncs_total", "counter", &self.state_resyncs);
        line("slow_clients_dropped_total", "counter", &self.slow_clients_dropped);
        let by_method = self.shadow_by_method.lock().unwrap();
        if !by_method.is_empty() {
            let _ = writeln!(out, "# TYPE playmat_gateway_shadow_method_total counter");
            for (method, (same, different)) in by_method.iter() {
                let _ =
                    writeln!(out, "playmat_gateway_shadow_method_total{{method=\"{method}\",result=\"same\"}} {same}");
                let _ = writeln!(
                    out,
                    "playmat_gateway_shadow_method_total{{method=\"{method}\",result=\"different\"}} {different}"
                );
            }
        }
        drop(by_method);
        let declined = self.declined_by_method.lock().unwrap();
        if !declined.is_empty() {
            let _ = writeln!(out, "# TYPE playmat_gateway_declined_method_total counter");
            for (method, count) in declined.iter() {
                let _ = writeln!(out, "playmat_gateway_declined_method_total{{method=\"{method}\"}} {count}");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declined_by_method() {
        let metrics = Metrics::default();
        metrics.declined("searchCards");
        metrics.declined("searchCards");
        metrics.declined("deckImportFromUrl");
        let text = metrics.render();
        assert!(text.contains("playmat_gateway_declined_total 3"));
        assert!(text.contains("playmat_gateway_declined_method_total{method=\"searchCards\"} 2"));
        assert!(text.contains("playmat_gateway_declined_method_total{method=\"deckImportFromUrl\"} 1"));
    }
}
