use crate::clients::models::{CapabilitySource, UnifyRouteMode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectiveConfigMode {
    Unify,
    Hosted,
    Transparent,
}

impl EffectiveConfigMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unify" => Some(Self::Unify),
            "hosted" => Some(Self::Hosted),
            "transparent" => Some(Self::Transparent),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceParticipation {
    Managed,
    Native,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileScopePolicy {
    Ignored,
    Activated,
    Selected,
    Custom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectExposurePolicy {
    Ignored,
    None,
    ServerLevel,
    CapabilityLevel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinSurfaceSet {
    None,
    Unify,
    Hosted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceCompositionPolicy {
    pub participation: SurfaceParticipation,
    pub profile_scope: ProfileScopePolicy,
    pub direct_exposure: DirectExposurePolicy,
    pub builtins: BuiltinSurfaceSet,
}

const fn profile_scope_policy(capability_source: CapabilitySource) -> ProfileScopePolicy {
    match capability_source {
        CapabilitySource::Activated => ProfileScopePolicy::Activated,
        CapabilitySource::Profiles => ProfileScopePolicy::Selected,
        CapabilitySource::Custom => ProfileScopePolicy::Custom,
    }
}

pub fn resolve_surface_composition_policy(
    mode: EffectiveConfigMode,
    capability_source: CapabilitySource,
    unify_route_mode: UnifyRouteMode,
) -> SurfaceCompositionPolicy {
    match mode {
        EffectiveConfigMode::Unify => SurfaceCompositionPolicy {
            participation: SurfaceParticipation::Managed,
            profile_scope: ProfileScopePolicy::Ignored,
            direct_exposure: match unify_route_mode {
                UnifyRouteMode::BrokerOnly => DirectExposurePolicy::None,
                UnifyRouteMode::ServerLevel => DirectExposurePolicy::ServerLevel,
                UnifyRouteMode::CapabilityLevel => DirectExposurePolicy::CapabilityLevel,
            },
            builtins: BuiltinSurfaceSet::Unify,
        },
        EffectiveConfigMode::Hosted => SurfaceCompositionPolicy {
            participation: SurfaceParticipation::Managed,
            profile_scope: profile_scope_policy(capability_source),
            direct_exposure: DirectExposurePolicy::Ignored,
            builtins: match capability_source {
                CapabilitySource::Profiles => BuiltinSurfaceSet::Hosted,
                CapabilitySource::Activated | CapabilitySource::Custom => BuiltinSurfaceSet::None,
            },
        },
        EffectiveConfigMode::Transparent => SurfaceCompositionPolicy {
            participation: SurfaceParticipation::Native,
            profile_scope: profile_scope_policy(capability_source),
            direct_exposure: DirectExposurePolicy::Ignored,
            builtins: BuiltinSurfaceSet::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::clients::models::{CapabilitySource, UnifyRouteMode};

    use super::{
        BuiltinSurfaceSet, DirectExposurePolicy, EffectiveConfigMode, ProfileScopePolicy, SurfaceParticipation,
        resolve_surface_composition_policy,
    };

    #[test]
    fn composition_policy_keeps_mode_profile_and_direct_exposure_boundaries_exhaustive() {
        let sources = [
            (CapabilitySource::Activated, ProfileScopePolicy::Activated),
            (CapabilitySource::Profiles, ProfileScopePolicy::Selected),
            (CapabilitySource::Custom, ProfileScopePolicy::Custom),
        ];
        let routes = [
            (UnifyRouteMode::BrokerOnly, DirectExposurePolicy::None),
            (UnifyRouteMode::ServerLevel, DirectExposurePolicy::ServerLevel),
            (UnifyRouteMode::CapabilityLevel, DirectExposurePolicy::CapabilityLevel),
        ];

        for (source, hosted_profile_scope) in sources {
            for (route_mode, unify_direct_exposure) in routes {
                let unify = resolve_surface_composition_policy(EffectiveConfigMode::Unify, source, route_mode);
                assert_eq!(unify.participation, SurfaceParticipation::Managed);
                assert_eq!(unify.profile_scope, ProfileScopePolicy::Ignored);
                assert_eq!(unify.direct_exposure, unify_direct_exposure);
                assert_eq!(unify.builtins, BuiltinSurfaceSet::Unify);

                let hosted = resolve_surface_composition_policy(EffectiveConfigMode::Hosted, source, route_mode);
                assert_eq!(hosted.participation, SurfaceParticipation::Managed);
                assert_eq!(hosted.profile_scope, hosted_profile_scope);
                assert_eq!(hosted.direct_exposure, DirectExposurePolicy::Ignored);
                assert_eq!(
                    hosted.builtins,
                    if source == CapabilitySource::Profiles {
                        BuiltinSurfaceSet::Hosted
                    } else {
                        BuiltinSurfaceSet::None
                    }
                );

                let transparent =
                    resolve_surface_composition_policy(EffectiveConfigMode::Transparent, source, route_mode);
                assert_eq!(transparent.participation, SurfaceParticipation::Native);
                assert_eq!(transparent.profile_scope, hosted_profile_scope);
                assert_eq!(transparent.direct_exposure, DirectExposurePolicy::Ignored);
                assert_eq!(transparent.builtins, BuiltinSurfaceSet::None);
            }
        }
    }
}
