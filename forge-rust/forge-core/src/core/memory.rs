//! Memory tracking and budget management

use std::sync::atomic::{AtomicUsize, Ordering};

/// Tracks memory usage and enforces budgets
#[derive(Debug)]
pub struct MemoryTracker {
    /// Current allocated bytes
    current_bytes: AtomicUsize,
    /// Peak allocated bytes
    peak_bytes: AtomicUsize,
    /// Maximum allowed bytes (0 = unlimited)
    budget_bytes: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker with optional budget
    pub fn new(budget_mb: Option<usize>) -> Self {
        Self {
            current_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
            budget_bytes: budget_mb.unwrap_or(0) * 1024 * 1024,
        }
    }

    /// Record an allocation
    pub fn alloc(&self, bytes: usize) -> bool {
        let new_current = self.current_bytes.fetch_add(bytes, Ordering::SeqCst) + bytes;

        // Update peak if needed
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while new_current > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak,
                new_current,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        // Check budget
        if self.budget_bytes > 0 && new_current > self.budget_bytes {
            // Over budget - still allocated but return false
            log::warn!(
                "Memory budget exceeded: {} MB / {} MB",
                new_current / (1024 * 1024),
                self.budget_bytes / (1024 * 1024)
            );
            return false;
        }

        true
    }

    /// Record a deallocation
    pub fn dealloc(&self, bytes: usize) {
        self.current_bytes.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Get current memory usage in MB
    pub fn current_mb(&self) -> f64 {
        self.current_bytes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in MB
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0)
    }

    /// Get remaining budget in MB (0 if unlimited)
    pub fn remaining_mb(&self) -> f64 {
        if self.budget_bytes == 0 {
            return 0.0;
        }
        let current = self.current_bytes.load(Ordering::Relaxed);
        if current >= self.budget_bytes {
            return 0.0;
        }
        (self.budget_bytes - current) as f64 / (1024.0 * 1024.0)
    }

    /// Check if allocation would fit within budget
    pub fn would_fit(&self, bytes: usize) -> bool {
        if self.budget_bytes == 0 {
            return true;
        }
        self.current_bytes.load(Ordering::Relaxed) + bytes <= self.budget_bytes
    }

    /// Reset tracking (does not actually free memory)
    pub fn reset(&self) {
        self.current_bytes.store(0, Ordering::SeqCst);
        self.peak_bytes.store(0, Ordering::SeqCst);
    }

    /// Log current memory status
    pub fn log_status(&self, tag: &str) {
        log::info!(
            "MEM[{}]: current={:.1} MB, peak={:.1} MB",
            tag,
            self.current_mb(),
            self.peak_mb()
        );
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking() {
        let tracker = MemoryTracker::new(Some(100)); // 100 MB budget

        tracker.alloc(50 * 1024 * 1024); // 50 MB
        assert!((tracker.current_mb() - 50.0).abs() < 0.1);

        tracker.alloc(30 * 1024 * 1024); // +30 MB = 80 MB
        assert!((tracker.current_mb() - 80.0).abs() < 0.1);

        tracker.dealloc(50 * 1024 * 1024); // -50 MB = 30 MB
        assert!((tracker.current_mb() - 30.0).abs() < 0.1);

        // Peak should still be 80 MB
        assert!((tracker.peak_mb() - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_budget_exceeded() {
        let tracker = MemoryTracker::new(Some(100)); // 100 MB budget

        assert!(tracker.alloc(50 * 1024 * 1024)); // OK
        assert!(!tracker.alloc(60 * 1024 * 1024)); // Exceeds budget
    }
}
