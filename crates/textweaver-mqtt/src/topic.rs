//! MQTT topic names and filters: checking them, matching a message's topic
//! against a filter (the quiet hint, and the broker's routing and access
//! rules), and whether one filter lies within another (the broker's
//! subscription rules).

/// Why a filter cannot be subscribed to, in words, or `None` when it is
/// valid: not empty, `#` only as the whole last level, `+` only as a whole
/// level, no NUL, at most 65535 bytes.
pub fn filter_problem(filter: &str) -> Option<&'static str> {
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
pub fn matches(filter: &str, topic: &str) -> bool {
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

/// Why a topic name cannot be published to, in words, or `None` when it is
/// valid: a filter's rules, and no `+` or `#` at all.
pub fn name_problem(name: &str) -> Option<&'static str> {
    if name.contains(['+', '#']) {
        return Some("a topic to publish to may not hold + or #");
    }
    filter_problem(name)
}

/// Whether every topic `inner` matches is also matched by `outer`, so a
/// subscription to `inner` asks for nothing `outer` does not allow. Both
/// must be valid filters. A wildcard at the first level of `outer` never
/// covers topics starting with `$`, as in [`matches()`].
pub fn filter_within(inner: &str, outer: &str) -> bool {
    if (outer.starts_with('+') || outer.starts_with('#')) && inner.starts_with('$') {
        return false;
    }
    let mut i = inner.split('/');
    let mut o = outer.split('/');
    loop {
        let (a, b) = (o.next(), i.next());
        match (a, b) {
            (Some("#"), _) => return true,
            (Some("+"), Some(l)) if l != "#" => {}
            (Some(x), Some(y)) if x == y && x != "+" && x != "#" => {}
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
    fn filters_within_filters() {
        assert!(filter_within("netlog/checkins", "netlog/#"));
        assert!(filter_within("netlog/#", "netlog/#"));
        assert!(filter_within("netlog", "netlog/#"));
        assert!(filter_within("netlog/+/x", "netlog/#"));
        assert!(filter_within("netlog/+", "netlog/+"));
        assert!(filter_within("netlog/a", "netlog/+"));
        assert!(!filter_within("netlog/#", "netlog/+"));
        assert!(!filter_within("netlog/a/b", "netlog/+"));
        assert!(!filter_within("#", "netlog/#"));
        assert!(!filter_within("+/checkins", "netlog/+"));
        assert!(!filter_within("other", "netlog/#"));
        assert!(filter_within("#", "#"));
        assert!(filter_within("+/x", "#"));
        assert!(!filter_within("$SYS/x", "#"));
        assert!(!filter_within("$SYS/x", "+/x"));
        assert!(filter_within("$SYS/x", "$SYS/#"));
    }

    #[test]
    fn names_have_no_wildcards() {
        assert_eq!(name_problem("netlog/checkins"), None);
        assert!(name_problem("netlog/+").is_some());
        assert!(name_problem("netlog/#").is_some());
        assert!(name_problem("").is_some());
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
