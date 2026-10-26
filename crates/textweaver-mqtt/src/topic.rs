//! MQTT topic filters: checking the configured ones, and matching a
//! message's topic against them for the quiet hint.

/// Why a filter cannot be subscribed to, in words, or `None` when it is
/// valid: not empty, `#` only as the whole last level, `+` only as a whole
/// level, no NUL, at most 65535 bytes.
pub(crate) fn filter_problem(filter: &str) -> Option<&'static str> {
    if filter.is_empty() {
        return Some("a topic is empty");
    }
    if filter.len() > usize::from(u16::MAX) {
        return Some("a topic is longer than 65535 bytes");
    }
    if filter.contains('\0') {
        return Some("a topic holds a NUL character");
    }
    let levels: Vec<&str> = filter.split('/').collect();
    for (i, level) in levels.iter().enumerate() {
        if level.contains('#') && (*level != "#" || i + 1 != levels.len()) {
            return Some("# may only be a whole last level of a topic");
        }
        if level.contains('+') && *level != "+" {
            return Some("+ may only be a whole level of a topic");
        }
    }
    None
}

/// Whether `topic` matches `filter`, with `+` for one level and `#` for
/// the rest. Topics starting with `$` match no wildcard at the first
/// level, as brokers do.
pub(crate) fn matches(filter: &str, topic: &str) -> bool {
    if topic.starts_with('$') && (filter.starts_with('+') || filter.starts_with('#')) {
        return false;
    }
    let mut f = filter.split('/');
    let mut t = topic.split('/');
    loop {
        match (f.next(), t.next()) {
            (Some("#"), _) => return true,
            (Some("+"), Some(_)) => {}
            (Some(a), Some(b)) if a == b => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_match_as_brokers_do() {
        assert!(matches("netlog/#", "netlog/checkins"));
        assert!(matches("netlog/#", "netlog"));
        assert!(matches("netlog/+/a", "netlog/x/a"));
        assert!(!matches("netlog/+", "netlog/x/a"));
        assert!(matches("a/b", "a/b"));
        assert!(!matches("a/b", "a/c"));
        assert!(!matches("#", "$SYS/broker"));
        assert!(matches("$SYS/#", "$SYS/broker"));
    }

    #[test]
    fn bad_filters_are_named() {
        assert_eq!(filter_problem("netlog/#"), None);
        assert_eq!(filter_problem("+/x/+"), None);
        assert!(filter_problem("").is_some());
        assert!(filter_problem("a/#/b").is_some());
        assert!(filter_problem("a/b#").is_some());
        assert!(filter_problem("a/b+").is_some());
    }
}
