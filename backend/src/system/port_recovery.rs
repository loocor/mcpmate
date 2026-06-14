use std::net::{Ipv4Addr, TcpListener};

use anyhow::{Result, anyhow};

pub const LOCALHOST_PORT_RECOVERY_SCAN_LIMIT: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalhostPortSelection {
    pub api_port: u16,
    pub mcp_port: u16,
}

impl LocalhostPortSelection {
    pub fn changed_from(
        &self,
        preferred_api_port: u16,
        preferred_mcp_port: u16,
    ) -> bool {
        self.api_port != preferred_api_port || self.mcp_port != preferred_mcp_port
    }
}

pub fn recover_available_localhost_ports(
    preferred_api_port: u16,
    preferred_mcp_port: u16,
) -> Result<LocalhostPortSelection> {
    recover_available_localhost_ports_with(
        preferred_api_port,
        preferred_mcp_port,
        LOCALHOST_PORT_RECOVERY_SCAN_LIMIT,
        is_localhost_port_available,
    )
}

pub fn recover_available_localhost_ports_selectively(
    preferred_api_port: u16,
    preferred_mcp_port: u16,
    recover_api_port: bool,
    recover_mcp_port: bool,
) -> Result<LocalhostPortSelection> {
    recover_available_localhost_ports_selectively_with(
        preferred_api_port,
        preferred_mcp_port,
        recover_api_port,
        recover_mcp_port,
        LOCALHOST_PORT_RECOVERY_SCAN_LIMIT,
        is_localhost_port_available,
    )
}

pub fn recover_available_localhost_ports_with(
    preferred_api_port: u16,
    preferred_mcp_port: u16,
    scan_limit: u16,
    is_available: impl FnMut(u16) -> bool,
) -> Result<LocalhostPortSelection> {
    recover_available_localhost_ports_selectively_with(
        preferred_api_port,
        preferred_mcp_port,
        true,
        true,
        scan_limit,
        is_available,
    )
}

pub fn recover_available_localhost_ports_selectively_with(
    preferred_api_port: u16,
    preferred_mcp_port: u16,
    recover_api_port: bool,
    recover_mcp_port: bool,
    scan_limit: u16,
    mut is_available: impl FnMut(u16) -> bool,
) -> Result<LocalhostPortSelection> {
    if preferred_api_port == 0 || preferred_mcp_port == 0 {
        return Err(anyhow!("localhost port recovery requires non-zero ports"));
    }

    if scan_limit == 0 {
        return Err(anyhow!("localhost port recovery scan limit must be greater than zero"));
    }

    let api_port = if recover_api_port {
        let reserved_port = (!recover_mcp_port).then_some(preferred_mcp_port);
        recover_port("API", preferred_api_port, scan_limit, &mut is_available, reserved_port)?
    } else {
        preferred_api_port
    };

    let mcp_port = if recover_mcp_port {
        recover_port("MCP", preferred_mcp_port, scan_limit, &mut is_available, Some(api_port))?
    } else {
        preferred_mcp_port
    };

    Ok(LocalhostPortSelection { api_port, mcp_port })
}

fn recover_port(
    port_kind: &str,
    preferred_port: u16,
    scan_limit: u16,
    is_available: &mut impl FnMut(u16) -> bool,
    reserved_port: Option<u16>,
) -> Result<u16> {
    next_available_port(preferred_port, scan_limit, is_available, reserved_port)
        .ok_or_else(|| anyhow!("no available {port_kind} port found from {preferred_port} within {scan_limit} ports"))
}

fn next_available_port(
    preferred_port: u16,
    scan_limit: u16,
    is_available: &mut impl FnMut(u16) -> bool,
    reserved_port: Option<u16>,
) -> Option<u16> {
    for offset in 0..scan_limit {
        let candidate = preferred_port.checked_add(offset)?;
        if reserved_port == Some(candidate) {
            continue;
        }
        if is_available(candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_localhost_port_available(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn availability(occupied: &[u16]) -> impl FnMut(u16) -> bool {
        let occupied: HashSet<u16> = occupied.iter().copied().collect();
        move |port| !occupied.contains(&port)
    }

    #[test]
    fn keeps_available_preferred_ports() {
        let selection = recover_available_localhost_ports_with(8080, 8000, 10, availability(&[])).unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8080,
                mcp_port: 8000
            }
        );
        assert!(!selection.changed_from(8080, 8000));
    }

    #[test]
    fn advances_occupied_api_port_only() {
        let selection = recover_available_localhost_ports_with(8080, 8000, 10, availability(&[8080])).unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8081,
                mcp_port: 8000
            }
        );
        assert!(selection.changed_from(8080, 8000));
    }

    #[test]
    fn advances_occupied_api_and_mcp_ports_independently() {
        let selection = recover_available_localhost_ports_with(8080, 8000, 10, availability(&[8080, 8000])).unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8081,
                mcp_port: 8001
            }
        );
    }

    #[test]
    fn never_selects_same_api_and_mcp_port() {
        let selection = recover_available_localhost_ports_with(8080, 8080, 10, availability(&[])).unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8080,
                mcp_port: 8081
            }
        );
    }

    #[test]
    fn reports_when_scan_window_has_no_available_api_port() {
        let err = recover_available_localhost_ports_with(8080, 8000, 2, availability(&[8080, 8081])).unwrap_err();

        assert!(err.to_string().contains("available API port"));
    }

    #[test]
    fn selectively_recovers_only_api_port() {
        let selection =
            recover_available_localhost_ports_selectively_with(8080, 8000, true, false, 10, availability(&[8080]))
                .unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8081,
                mcp_port: 8000
            }
        );
    }

    #[test]
    fn selectively_recovers_only_mcp_port() {
        let selection =
            recover_available_localhost_ports_selectively_with(8080, 8000, false, true, 10, availability(&[8000]))
                .unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8080,
                mcp_port: 8001
            }
        );
    }

    #[test]
    fn selective_api_recovery_reserves_explicit_mcp_port() {
        let selection =
            recover_available_localhost_ports_selectively_with(8080, 8081, true, false, 10, availability(&[8080]))
                .unwrap();

        assert_eq!(
            selection,
            LocalhostPortSelection {
                api_port: 8082,
                mcp_port: 8081
            }
        );
    }
}
