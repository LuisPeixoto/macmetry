//! What an assistant is not sent. The command line and the app show their owner everything; an assistant gets what it
//! needs to answer questions about performance, and nothing that identifies the machine on a network.

use macmetry_core::Snapshot;

/// Removes the addresses of the network interfaces. Process names and users stay: they are what explains a slow Mac.
pub fn for_assistant(mut snapshot: Snapshot) -> Snapshot {
    if let Some(network) = snapshot.network.as_mut() {
        for interface in &mut network.interfaces {
            interface.addresses.clear();
        }
    }
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::{NetInterface, NetworkStats};

    #[test]
    fn network_addresses_are_removed_and_the_rest_is_kept() {
        let snapshot = Snapshot {
            network: Some(NetworkStats {
                interfaces: vec![NetInterface {
                    name: "en0".into(),
                    rx_bps: 5.0,
                    addresses: vec!["192.168.1.2".into()],
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        let cleaned = for_assistant(snapshot);
        let interface = &cleaned.network.unwrap().interfaces[0];
        assert!(interface.addresses.is_empty());
        assert_eq!((interface.name.as_str(), interface.rx_bps), ("en0", 5.0));
    }

    #[test]
    fn a_snapshot_without_a_network_is_unchanged() {
        assert_eq!(for_assistant(Snapshot::default()), Snapshot::default());
    }
}
