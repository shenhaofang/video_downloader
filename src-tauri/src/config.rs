use crate::models::AppConfig;

pub fn normalize_concurrency(value: u8) -> u8 {
    value.clamp(1, 8)
}

pub fn normalize_persisted_concurrency(value: i64) -> u8 {
    value.clamp(1, 8) as u8
}

pub fn with_normalized_concurrency(mut config: AppConfig) -> AppConfig {
    config.concurrency = normalize_concurrency(config.concurrency);
    config
}

#[cfg(test)]
mod tests {
    use super::{normalize_concurrency, with_normalized_concurrency};
    use crate::models::AppConfig;

    #[test]
    fn clamps_concurrency_to_supported_range() {
        assert_eq!(normalize_concurrency(0), 1);
        assert_eq!(normalize_concurrency(2), 2);
        assert_eq!(normalize_concurrency(99), 8);
    }

    #[test]
    fn clamps_persisted_concurrency_to_supported_range() {
        assert_eq!(super::normalize_persisted_concurrency(-1), 1);
        assert_eq!(super::normalize_persisted_concurrency(2), 2);
        assert_eq!(super::normalize_persisted_concurrency(300), 8);
    }

    #[test]
    fn returns_config_with_normalized_concurrency() {
        let config = AppConfig {
            concurrency: 99,
            ..AppConfig::default()
        };

        let normalized = with_normalized_concurrency(config);

        assert_eq!(normalized.concurrency, 8);
    }
}
