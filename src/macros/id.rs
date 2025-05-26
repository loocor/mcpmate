// use proc_macro2::{TokenStream}; // Not needed for macro_rules!
// use quote::quote; // Not needed for macro_rules!
// use syn::{LitInt, parse_macro_input}; // Not needed for macro_rules!

/// generate unique id with prefix
///
/// # Parameters
/// * `prefix` - ID prefix (string literal)
/// * `length` - length of the random part (number literal, optional, defaults to 12)
///
/// # Examples
/// ```rust,no_run
/// use mcpmate::generate_id;
///
/// // With custom length
/// let id = generate_id!("srv", 10);
/// assert!(id.starts_with("srv_"));
/// assert_eq!(id.len(), 4 + 10); // "srv_" + 10 random characters
///
/// // With default length (12)
/// let id = generate_id!("suit");
/// assert!(id.starts_with("suit_"));
/// assert_eq!(id.len(), 5 + 12); // "suit_" + 12 random characters
/// ```
#[macro_export]
macro_rules! generate_id {
    // Pattern with explicit length
    ($prefix:literal, $length:literal) => {{
        use nanoid::nanoid;
        format!("{}_{}", $prefix, nanoid!($length))
    }};
    // Pattern with default length (12)
    ($prefix:literal) => {{
        use nanoid::nanoid;
        format!("{}_{}", $prefix, nanoid!(12))
    }};
}

/// generate pure random ID (no prefix)
///
/// # Parameters
/// * `length` - ID length (number literal)
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
        nanoid!($length)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_generate_id_with_length() {
        let id = crate::generate_id!("test", 8);
        assert!(id.starts_with("test_"));
        assert_eq!(id.len(), 13); // "test_" (5) + 8 = 13
    }

    #[test]
    fn test_generate_id_default_length() {
        let id = crate::generate_id!("suit");
        assert!(id.starts_with("suit_"));
        assert_eq!(id.len(), 17); // "suit_" (5) + 12 = 17
    }

    #[test]
    fn test_generate_raw_id() {
        let id = crate::generate_raw_id!(12);
        assert_eq!(id.len(), 12);
    }
}
