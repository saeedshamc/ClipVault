//! Retention policy evaluation helpers.

use crate::config::Config;

/// Inputs observed from the store when deciding what to delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionSnapshot {
    pub item_count: u64,
    pub total_bytes: u64,
}

/// Whether the store currently exceeds a retention limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPressure {
    pub over_count: bool,
    pub over_bytes: bool,
}

impl RetentionPressure {
    pub fn any(self) -> bool {
        self.over_count || self.over_bytes
    }
}

/// Compare live totals against configured limits.
pub fn evaluate_pressure(config: &Config, snapshot: RetentionSnapshot) -> RetentionPressure {
    RetentionPressure {
        over_count: snapshot.item_count > config.limits.max_items,
        over_bytes: snapshot.total_bytes > config.limits.max_total_bytes,
    }
}

/// Return true when `total_size` exceeds the per-item capture limit.
pub fn exceeds_item_limit(config: &Config, total_size: u64) -> bool {
    total_size > config.limits.max_item_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_count_and_byte_pressure() {
        let mut config = Config::default();
        config.limits.max_items = 2;
        config.limits.max_total_bytes = 100;
        let pressure = evaluate_pressure(
            &config,
            RetentionSnapshot {
                item_count: 3,
                total_bytes: 150,
            },
        );
        assert!(pressure.over_count);
        assert!(pressure.over_bytes);
        assert!(pressure.any());
    }

    #[test]
    fn item_limit_boundary() {
        let mut config = Config::default();
        config.limits.max_item_bytes = 10;
        assert!(!exceeds_item_limit(&config, 10));
        assert!(exceeds_item_limit(&config, 11));
    }
}
