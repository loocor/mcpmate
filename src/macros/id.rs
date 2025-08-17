// Internal helper to create alphabet without underscore and hyphen
pub fn create_safe_alphabet() -> Vec<char> {
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        .chars()
        .collect()
}

/// generate unique id with prefix
///
/// # Parameters
/// * `prefix` - ID prefix (string literal)
/// * `length` - length of the random part (number literal, optional, defaults to 12)
///
/// # Notes
/// Generated IDs will not contain underscore (_) or hyphen (-) characters.
///
/// # Examples
/// ```rust,no_run
/// use mcpmate::generate_id;
///
/// // With custom length
/// let id = generate_id!("srv", 10);
/// assert!(id.starts_with("SRV"));
/// assert_eq!(id.len(), 3 + 10); // "SRV" + 10 random characters
///
/// // With default length (12)
/// let id = generate_id!("suit");
/// assert!(id.starts_with("SUIT"));
/// assert_eq!(id.len(), 4 + 12); // "SUIT" + 12 random characters
/// ```
#[macro_export]
macro_rules! generate_id {
    // Pattern with explicit length
    ($prefix:literal, $length:literal) => {{
        use nanoid::nanoid;
        let alphabet = $crate::macros::id::create_safe_alphabet();
        format!("{}{}", $prefix.to_uppercase(), nanoid!($length, &alphabet))
    }};
    // Pattern with default length (12)
    ($prefix:literal) => {
        $crate::generate_id!($prefix, 12)
    };
}

/// generate pure random ID (no prefix)
///
/// # Parameters
/// * `length` - ID length (number literal)
///
/// # Notes
/// Generated IDs will not contain underscore (_) or hyphen (-) characters.
///
/// # Example
/// ```rust,no_run
/// use mcpmate::generate_raw_id;
/// let id = generate_raw_id!(16);
/// assert_eq!(id.len(), 16);
/// ```
#[macro_export]
macro_rules! generate_raw_id {
    ($length:literal) => {{
        use nanoid::nanoid;
        let alphabet = $crate::macros::id::create_safe_alphabet();
        nanoid!($length, &alphabet)
    }};
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_generate_id_no_underscore_hyphen() {
        // Test that generated IDs don't contain _ or -
        for _ in 0..100 {
            let id = crate::generate_id!("test");
            assert!(!id.contains('_'), "ID should not contain underscore: {}", id);
            assert!(!id.contains('-'), "ID should not contain hyphen: {}", id);
            assert!(id.starts_with("TEST"), "ID should start with prefix: {}", id);
            assert_eq!(id.len(), 4 + 12, "ID should be 16 characters total: {}", id); // "TEST" + 12 chars
        }
    }

    #[test]
    fn test_generate_id_custom_length() {
        for _ in 0..50 {
            let id = crate::generate_id!("srv", 8);
            assert!(!id.contains('_'), "ID should not contain underscore: {}", id);
            assert!(!id.contains('-'), "ID should not contain hyphen: {}", id);
            assert!(id.starts_with("SRV"), "ID should start with prefix: {}", id);
            assert_eq!(id.len(), 3 + 8, "ID should be 11 characters total: {}", id); // "SRV" + 8 chars
        }
    }

    #[test]
    fn test_generate_raw_id_no_underscore_hyphen() {
        for _ in 0..50 {
            let id = crate::generate_raw_id!(16);
            assert!(!id.contains('_'), "Raw ID should not contain underscore: {}", id);
            assert!(!id.contains('-'), "Raw ID should not contain hyphen: {}", id);
            assert_eq!(id.len(), 16, "Raw ID should be 16 characters: {}", id);
        }
    }
}
