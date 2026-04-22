//! Port-scanning and -assignment helpers.
//!
//! Two sources of "in use" ports are consulted:
//!  * the persistent [`PortRegistry`] in `~/.config/creo/ports.toml`
//!  * a live `TcpListener::bind` probe to detect ports occupied by other
//!    processes right now.

use std::collections::HashMap;
use std::net::TcpListener;

use crate::config::ports::PortRegistry;

/// Returns true if no process currently listens on the given port (we can
/// successfully bind to `0.0.0.0:<port>`). Avoids the need for an external
/// `ss` / `netstat` dependency.
pub fn is_port_free(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}

/// Find the next free port at or above `base`, skipping any port that is
/// either already bound or reserved in the persistent registry.
///
/// A port is considered "free" only if both checks pass.
pub fn find_free_port(base: u16, registry: &PortRegistry) -> u16 {
    find_free_port_excluding(base, registry, &[])
}

/// Like [`find_free_port`] but additionally skips ports listed in `extra`
/// (used when allocating multiple ports in one batch so we don't return
/// duplicates).
pub fn find_free_port_excluding(base: u16, registry: &PortRegistry, extra: &[u16]) -> u16 {
    let reserved = registry.reserved();
    let mut port = base.max(1);
    loop {
        if !reserved.contains(&port) && !extra.contains(&port) && is_port_free(port) {
            return port;
        }
        port = port.saturating_add(1);
        if port == 0 {
            // Wrap-around safety – should never hit in practice.
            return base;
        }
    }
}

/// Assign one free port per service description, persisting reservations
/// in the registry as we go. Returns a `service_name -> port` map.
///
/// `services` is a list of `(service_name, port_offset)` pairs; the
/// offset is added to `base_port` to determine the *preferred* starting
/// point for that service. The next free port at or above the preference
/// is then chosen.
pub fn assign_ports(
    project: &str,
    services: &[(String, u16)],
    registry: &mut PortRegistry,
    base_port: u16,
) -> HashMap<String, u16> {
    let mut assigned: HashMap<String, u16> = HashMap::new();
    let mut taken_now: Vec<u16> = Vec::new();
    for (svc, offset) in services {
        let preferred = base_port.saturating_add(*offset);
        let port = find_free_port_excluding(preferred, registry, &taken_now);
        registry.reserve(format!("{project}-{svc}"), port);
        taken_now.push(port);
        assigned.insert(svc.clone(), port);
    }
    assigned
}
