//! Fixed-size histories that feed the sparklines.

use std::collections::VecDeque;

pub const HISTORY_LEN: usize = 300;

#[derive(Debug, Clone)]
pub struct History {
    cap: usize,
    buf: VecDeque<u64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        History {
            cap,
            buf: VecDeque::with_capacity(cap),
        }
    }

    pub fn push(&mut self, v: u64) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    /// The newest `n` samples, oldest first, so a sparkline of width `n`
    /// always ends at "now" on its right edge.
    pub fn tail(&self, n: usize) -> Vec<u64> {
        let skip = self.buf.len().saturating_sub(n);
        self.buf.iter().skip(skip).copied().collect()
    }

    pub fn last(&self) -> u64 {
        self.buf.back().copied().unwrap_or(0)
    }
}

impl Default for History {
    fn default() -> Self {
        History::new(HISTORY_LEN)
    }
}

/// Bytes per second from a byte count gathered over `secs` seconds.
pub fn rate(bytes: u64, secs: f64) -> f64 {
    if secs <= 0.0 {
        0.0
    } else {
        bytes as f64 / secs
    }
}

#[derive(Debug, Default, Clone)]
pub struct Histories {
    /// Percent, 0..=100.
    pub cpu: History,
    /// Percent, 0..=100.
    pub mem: History,
    /// Bytes per second, summed over every interface except loopback.
    pub net_rx: History,
    pub net_tx: History,
    /// Bytes per second, summed over every disk.
    pub disk_read: History,
    pub disk_write: History,
    /// Percent of the first GPU, 0..=100.
    pub gpu: History,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest() {
        let mut h = History::new(3);
        for v in 1..=5 {
            h.push(v);
        }
        assert_eq!(h.tail(10), vec![3, 4, 5]);
        assert_eq!(h.tail(usize::MAX).len(), 3);
        assert_eq!(h.last(), 5);
    }

    #[test]
    fn tail_returns_newest() {
        let mut h = History::new(10);
        for v in 1..=5 {
            h.push(v);
        }
        assert_eq!(h.tail(2), vec![4, 5]);
    }

    #[test]
    fn rate_handles_zero_time() {
        assert_eq!(rate(1000, 0.0), 0.0);
        assert_eq!(rate(1000, 0.5), 2000.0);
    }
}
