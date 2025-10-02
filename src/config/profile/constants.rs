//! Profile-specific constants and helpers.

use crate::config::models::Profile;

/// Canonical slug for the system default shared profile.
pub const DEFAULT_PROFILE_SLUG: &str = "default-shared-profile";

/// Legacy name used before 2025-10-02; kept for backward compatibility during transition.
pub const LEGACY_DEFAULT_PROFILE_NAME: &str = "default";

/// Description applied to the system default profile when seeding the database.
pub const DEFAULT_PROFILE_DESCRIPTION: &str = "Default shared profile";

/// Returns true when the provided profile name matches the system default profile.
#[inline]
pub fn is_primary_default_name(name: &str) -> bool {
    name == DEFAULT_PROFILE_SLUG || name == LEGACY_DEFAULT_PROFILE_NAME
}

/// Returns true when the provided profile is the immutable system default profile.
#[inline]
pub fn is_primary_default_profile(profile: &Profile) -> bool {
    profile.name.as_str().eq(DEFAULT_PROFILE_SLUG) || profile.name.as_str().eq(LEGACY_DEFAULT_PROFILE_NAME)
}
