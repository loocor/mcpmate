use std::marker::PhantomData;
use std::fmt;

// Internal helper to create alphabet without underscore and hyphen
pub fn create_safe_alphabet() -> Vec<char> {
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        .chars()
        .collect()
}

/// Type-safe ID wrapper that prevents string mixing at compile time
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypedId<T> {
    id: String,
    _phantom: PhantomData<T>,
}

impl<T> TypedId<T> {
    /// Create a new typed ID from a string
    pub fn new(id: String) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    /// Get the underlying string ID
    pub fn as_str(&self) -> &str {
        &self.id
    }

    /// Convert to owned string
    pub fn into_string(self) -> String {
        self.id
    }
}

impl<T> fmt::Debug for TypedId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypedId({})", self.id)
    }
}

impl<T> fmt::Display for TypedId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T> AsRef<str> for TypedId<T> {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl<T> From<TypedId<T>> for String {
    fn from(typed_id: TypedId<T>) -> String {
        typed_id.id
    }
}

// Type markers for different ID types
pub struct Server;
pub struct Suit;
pub struct Session;
pub struct Connection;
pub struct Tool;
pub struct Resource;
pub struct Prompt;

// Type aliases for convenience
pub type ServerId = TypedId<Server>;
pub type SuitId = TypedId<Suit>;
pub type SessionId = TypedId<Session>;
pub type ConnectionId = TypedId<Connection>;
pub type ToolId = TypedId<Tool>;
pub type ResourceId = TypedId<Resource>;
pub type PromptId = TypedId<Prompt>;

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

/// Generate type-safe ID with compile-time type checking
///
/// # Parameters
/// * `prefix` - ID prefix (string literal)
/// * `type` - Type marker for compile-time safety
/// * `length` - length of the random part (optional, defaults to 12)
///
/// # Examples
/// ```rust,no_run
/// use mcpmate::{generate_typed_id, macros::id::{Server, Suit}};
///
/// let server_id = generate_typed_id!("srv", Server);
/// let suit_id = generate_typed_id!("suit", Suit, 8);
///
/// // Compile-time error: cannot mix different ID types
/// // let mixed = server_id == suit_id; // This won't compile!
/// ```
#[macro_export]
macro_rules! generate_typed_id {
    // Pattern with explicit length
    ($prefix:literal, $type:ty, $length:literal) => {{
        use nanoid::nanoid;
        let alphabet = $crate::macros::id::create_safe_alphabet();
        let id = format!("{}{}", $prefix.to_uppercase(), nanoid!($length, &alphabet));
        $crate::macros::id::TypedId::<$type>::new(id)
    }};
    // Pattern with default length (12)
    ($prefix:literal, $type:ty) => {
        $crate::generate_typed_id!($prefix, $type, 12)
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
    use super::*;

    #[test]
    fn test_typed_id_display_and_debug() {
        let connection_id: ConnectionId = generate_typed_id!("conn", Connection);

        // Test Display trait
        let display_str = format!("{}", connection_id);
        assert!(display_str.starts_with("CONN"));

        // Test Debug trait
        let debug_str = format!("{:?}", connection_id);
        assert!(debug_str.contains("TypedId"));
        assert!(debug_str.contains("CONN"));
    }

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
