//! Port-scanning and -assignment helpers.
//!
//! Two sources of "in use" ports are consulted:
//!  * the persistent [`PortRegistry`] in `~/.config/skap/ports.toml`
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
#[allow(dead_code)]
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
        if port == u16::MAX {
            // Exhausted the entire port space above `base` – give up and
            // hand back the original base rather than looping forever
            // (`saturating_add` would otherwise get stuck at u16::MAX).
            return base;
        }
        port += 1;
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
        registry.reserve(project, svc, port);
        taken_now.push(port);
        assigned.insert(svc.clone(), port);
    }
    assigned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_free_port_excluding_skips_reserved_ports() {
        let mut registry = PortRegistry::default();
        registry.reserve("proj", "a", 3000);
        registry.reserve("proj", "b", 3001);
        // 3000 and 3001 are reserved (not necessarily bound), so the
        // search should skip past both even though nothing is actually
        // listening on them.
        let port = find_free_port_excluding(3000, &registry, &[]);
        assert!(port >= 3002, "expected port >= 3002, got {port}");
    }

    #[test]
    fn find_free_port_excluding_skips_extra() {
        let registry = PortRegistry::default();
        let port = find_free_port_excluding(3000, &registry, &[3000, 3001, 3002]);
        assert!(port >= 3003, "expected port >= 3003, got {port}");
    }

    /// Regression test: searching from near the top of the u16 range must
    /// terminate instead of looping forever. Every port from `base` to
    /// `u16::MAX` is marked reserved, forcing the search to walk all the
    /// way to the top and hit the give-up path.
    #[test]
    fn find_free_port_excluding_terminates_near_u16_max() {
        let mut registry = PortRegistry::default();
        let base: u16 = 65530;
        for (i, port) in (base..=u16::MAX).enumerate() {
            registry.reserve("proj", &format!("svc{i}"), port);
        }
        // Must return (not hang) even though every candidate from `base`
        // upward is reserved.
        let port = find_free_port_excluding(base, &registry, &[]);
        assert_eq!(port, base, "should fall back to `base` when exhausted");
    }

    #[test]
    fn assign_ports_gives_distinct_ports_per_service() {
        let mut registry = PortRegistry::default();
        let services = vec![("app".to_string(), 0u16), ("db".to_string(), 1u16)];
        let assigned = assign_ports("proj", &services, &mut registry, 4000);
        assert_ne!(assigned["app"], assigned["db"]);
        assert_eq!(registry.reserved().len(), 2);
    }
}
