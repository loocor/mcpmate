//! Profile-specific constants and helpers.

use crate::common::profile::ProfileRole;
use crate::config::models::Profile;

/// Initial display name applied when seeding the default anchor profile.
pub const DEFAULT_ANCHOR_INITIAL_NAME: &str = "Default Anchor";

/// Description applied to the default anchor profile when seeding the database.
pub const DEFAULT_PROFILE_DESCRIPTION: &str = "Default anchor profile";

/// Role assigned to the system default anchor profile.
pub const DEFAULT_ANCHOR_ROLE: ProfileRole = ProfileRole::DefaultAnchor;

/// Returns true when the provided profile carries the default anchor role.
#[inline]
pub fn is_default_anchor_profile(profile: &Profile) -> bool {
    profile.role.is_default_anchor()
}
