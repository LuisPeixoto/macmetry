//! Traffic per network interface, from the kernel's 64-bit interface counters.

use crate::cf::{CFTypeRef, Owned, string_from};
use crate::error::{Result, SysError};
use macmetry_core::rates::per_second;
use macmetry_core::{InterfaceKind, NetInterface, NetworkStats};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

#[link(name = "SystemConfiguration", kind = "framework")]
unsafe extern "C" {
    fn SCNetworkInterfaceCopyAll() -> CFTypeRef;
    fn SCNetworkInterfaceGetBSDName(interface: CFTypeRef) -> CFTypeRef;
    fn SCNetworkInterfaceGetInterfaceType(interface: CFTypeRef) -> CFTypeRef;
}

/// Counters of one interface from an `NET_RT_IFLIST2` message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RawInterface {
    pub index: u32,
    pub flags: i32,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

/// Parses the routing socket buffer. Messages that are not interface info, are truncated or claim an impossible
/// length are skipped, so a corrupt buffer yields fewer interfaces instead of a crash or an endless loop.
pub fn parse_iflist2(buf: &[u8]) -> Vec<RawInterface> {
    let header = std::mem::size_of::<libc::if_msghdr2>();
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= buf.len() {
        let len = usize::from(u16::from_ne_bytes([buf[offset], buf[offset + 1]]));
        if len == 0 || offset + len > buf.len() {
            break;
        }
        if i32::from(buf[offset + 3]) == libc::RTM_IFINFO2 && len >= header {
            // SAFETY: `len >= header` and the message lies inside `buf`; the read is unaligned-safe and the struct
            // is plain integers.
            let msg: libc::if_msghdr2 = unsafe { std::ptr::read_unaligned(buf[offset..].as_ptr().cast()) };
            out.push(RawInterface {
                index: u32::from(msg.ifm_index),
                flags: msg.ifm_flags,
                rx_bytes: msg.ifm_data.ifi_ibytes,
                tx_bytes: msg.ifm_data.ifi_obytes,
                rx_packets: msg.ifm_data.ifi_ipackets,
                tx_packets: msg.ifm_data.ifi_opackets,
                rx_errors: msg.ifm_data.ifi_ierrors,
                tx_errors: msg.ifm_data.ifi_oerrors,
            });
        }
        offset += len;
    }
    out
}

/// Names the system gives to interfaces that are not a cable or a radio.
const VIRTUAL_PREFIXES: &[&str] =
    &["utun", "awdl", "llw", "bridge", "gif", "stf", "ap", "anpi", "ipsec", "ppp", "vmenet", "vnic"];

/// What an interface is. `config_type` is SystemConfiguration's type for it, when it knows the interface.
pub fn kind_of(name: &str, flags: i32, config_type: Option<&str>) -> InterfaceKind {
    if flags & libc::IFF_LOOPBACK != 0 || name.starts_with("lo") {
        return InterfaceKind::Loopback;
    }
    match config_type {
        Some("IEEE80211") => InterfaceKind::Wifi,
        Some("Ethernet") => InterfaceKind::Ethernet,
        Some("Bridge" | "VLAN" | "Bond" | "6to4" | "PPP" | "IPSec" | "VPN") => InterfaceKind::Virtual,
        _ if VIRTUAL_PREFIXES.iter().any(|p| name.starts_with(p)) => InterfaceKind::Virtual,
        _ => InterfaceKind::Other,
    }
}

pub fn read_raw() -> Result<Vec<RawInterface>> {
    let mut mib = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
    let mut len: libc::size_t = 0;
    // SAFETY: `mib` has 6 entries; a null output buffer asks only for the size.
    let rc = unsafe { libc::sysctl(mib.as_mut_ptr(), 6, std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return Err(SysError::os("sysctl NET_RT_IFLIST2"));
    }
    let mut buf = vec![0u8; len + len / 8 + 64];
    let mut got = buf.len();
    // SAFETY: `buf` has `got` writable bytes.
    let rc = unsafe { libc::sysctl(mib.as_mut_ptr(), 6, buf.as_mut_ptr().cast(), &mut got, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return Err(SysError::os("sysctl NET_RT_IFLIST2"));
    }
    buf.truncate(got);
    Ok(parse_iflist2(&buf))
}

fn interface_name(index: u32) -> Option<String> {
    let mut buf = [0 as libc::c_char; libc::IF_NAMESIZE];
    // SAFETY: `buf` is `IF_NAMESIZE` bytes, which is what the call requires.
    let ptr = unsafe { libc::if_indextoname(index, buf.as_mut_ptr()) };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: on success the buffer holds a NUL-terminated name.
    Some(unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
}

/// SystemConfiguration's type of each interface, by BSD name (`IEEE80211` for Wi-Fi, `Ethernet`, `Bridge`...).
fn config_types() -> HashMap<String, String> {
    let mut map = HashMap::new();
    // SAFETY: the call has no preconditions and returns a new array the caller owns (or null).
    let Some(all) = (unsafe { Owned::from_create(SCNetworkInterfaceCopyAll()) }) else { return map };
    for item in all.array_items() {
        // SAFETY: `item` is borrowed from `all`, which lives through this loop; both getters return borrowed strings.
        let (name, kind) = unsafe {
            (string_from(SCNetworkInterfaceGetBSDName(item)), string_from(SCNetworkInterfaceGetInterfaceType(item)))
        };
        if let (Some(name), Some(kind)) = (name, kind) {
            map.insert(name, kind);
        }
    }
    map
}

/// Addresses assigned to each interface. IPv6 link-local and multicast addresses are left out: they are the same
/// on every Mac and say nothing useful.
pub fn local_addresses() -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: `list` is a valid out pointer; the list is freed below.
    if unsafe { libc::getifaddrs(&mut list) } != 0 {
        return map;
    }
    let mut cursor = list;
    while !cursor.is_null() {
        // SAFETY: `cursor` is a node of the list returned by `getifaddrs`, which stays valid until `freeifaddrs`.
        let node = unsafe { &*cursor };
        cursor = node.ifa_next;
        if node.ifa_addr.is_null() {
            continue;
        }
        // SAFETY: `ifa_name` is a valid NUL-terminated string and `ifa_addr` points to a sockaddr.
        let (name, family) = unsafe {
            (
                std::ffi::CStr::from_ptr(node.ifa_name).to_string_lossy().into_owned(),
                i32::from((*node.ifa_addr).sa_family),
            )
        };
        let text = if family == libc::AF_INET {
            // SAFETY: an `AF_INET` address is a `sockaddr_in`.
            let sin = unsafe { &*node.ifa_addr.cast::<libc::sockaddr_in>() };
            Some(Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)).to_string())
        } else if family == libc::AF_INET6 {
            // SAFETY: an `AF_INET6` address is a `sockaddr_in6`.
            let sin6 = unsafe { &*node.ifa_addr.cast::<libc::sockaddr_in6>() };
            let ip = Ipv6Addr::from(sin6.sin6_addr.s6_addr);
            let link_local = ip.segments()[0] & 0xffc0 == 0xfe80;
            (!link_local && !ip.is_multicast()).then(|| ip.to_string())
        } else {
            None
        };
        if let Some(text) = text {
            map.entry(name).or_default().push(text);
        }
    }
    // SAFETY: `list` came from `getifaddrs` and is freed once.
    unsafe { libc::freeifaddrs(list) };
    map
}

/// Keeps the previous counters so each call returns rates since the last one.
pub struct NetworkCollector {
    previous: HashMap<u32, RawInterface>,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let previous = read_raw().unwrap_or_default().into_iter().map(|i| (i.index, i)).collect();
        Self { previous }
    }

    pub fn sample(&mut self, elapsed_ms: u64) -> Result<NetworkStats> {
        let current = read_raw()?;
        let types = config_types();
        let addresses = local_addresses();
        let mut interfaces: Vec<NetInterface> = current
            .iter()
            .filter_map(|now| {
                let name = interface_name(now.index)?;
                let before = self.previous.get(&now.index).unwrap_or(now);
                Some(NetInterface {
                    kind: kind_of(&name, now.flags, types.get(&name).map(String::as_str)),
                    up: now.flags & libc::IFF_UP != 0,
                    rx_bps: per_second(before.rx_bytes, now.rx_bytes, elapsed_ms),
                    tx_bps: per_second(before.tx_bytes, now.tx_bytes, elapsed_ms),
                    rx_packets_ps: per_second(before.rx_packets, now.rx_packets, elapsed_ms),
                    tx_packets_ps: per_second(before.tx_packets, now.tx_packets, elapsed_ms),
                    rx_errors: now.rx_errors,
                    tx_errors: now.tx_errors,
                    rx_total: now.rx_bytes,
                    tx_total: now.tx_bytes,
                    addresses: addresses.get(&name).cloned().unwrap_or_default(),
                    name,
                })
            })
            .collect();
        interfaces.sort_by(|a, b| a.name.cmp(&b.name));
        self.previous = current.into_iter().map(|i| (i.index, i)).collect();
        Ok(NetworkStats { interfaces })
    }
}

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds one interface message the way the kernel lays it out.
    fn message(index: u16, flags: i32, rx: u64, tx: u64) -> Vec<u8> {
        // SAFETY: a zeroed `if_msghdr2` is a valid value of plain integers.
        let mut msg: libc::if_msghdr2 = unsafe { std::mem::zeroed() };
        msg.ifm_msglen = std::mem::size_of::<libc::if_msghdr2>() as u16;
        msg.ifm_type = libc::RTM_IFINFO2 as u8;
        msg.ifm_index = index;
        msg.ifm_flags = flags;
        msg.ifm_data.ifi_ibytes = rx;
        msg.ifm_data.ifi_obytes = tx;
        msg.ifm_data.ifi_ierrors = 3;
        // SAFETY: the struct is plain integers, so viewing it as bytes is valid.
        unsafe {
            std::slice::from_raw_parts(
                (&msg as *const libc::if_msghdr2).cast::<u8>(),
                std::mem::size_of::<libc::if_msghdr2>(),
            )
        }
        .to_vec()
    }

    #[test]
    fn parses_interface_messages() {
        let mut buf = message(4, libc::IFF_UP, 1_000, 2_000);
        buf.extend(message(7, 0, 5, 6));
        let found = parse_iflist2(&buf);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].index, found[0].rx_bytes, found[0].tx_bytes, found[0].rx_errors), (4, 1_000, 2_000, 3));
        assert_eq!(found[1].index, 7);
    }

    #[test]
    fn skips_other_messages_and_survives_garbage() {
        let mut other = message(1, 0, 0, 0);
        other[3] = 0x01; // not RTM_IFINFO2
        assert!(parse_iflist2(&other).is_empty());

        let mut truncated = message(1, 0, 9, 9);
        truncated.truncate(truncated.len() - 10);
        assert!(parse_iflist2(&truncated).is_empty(), "a message that overruns the buffer is dropped");

        assert!(parse_iflist2(&[]).is_empty());
        assert!(parse_iflist2(&[0, 0, 0, 0, 0, 0, 0, 0]).is_empty(), "a zero length does not loop forever");
        assert!(parse_iflist2(&[0xff; 64]).is_empty());
    }

    #[test]
    fn classifies_interfaces() {
        assert_eq!(kind_of("lo0", libc::IFF_LOOPBACK, None), InterfaceKind::Loopback);
        assert_eq!(kind_of("en0", 0, Some("IEEE80211")), InterfaceKind::Wifi);
        assert_eq!(kind_of("en1", 0, Some("Ethernet")), InterfaceKind::Ethernet);
        assert_eq!(kind_of("bridge0", 0, Some("Bridge")), InterfaceKind::Virtual);
        assert_eq!(kind_of("utun3", 0, None), InterfaceKind::Virtual);
        assert_eq!(kind_of("awdl0", 0, None), InterfaceKind::Virtual);
        assert_eq!(kind_of("XHC20", 0, None), InterfaceKind::Other);
    }

    #[test]
    fn reads_this_macs_interfaces() {
        let mut collector = NetworkCollector::new();
        std::thread::sleep(std::time::Duration::from_millis(150));
        let stats = collector.sample(150).unwrap();
        let lo = stats.interfaces.iter().find(|i| i.name == "lo0").expect("the loopback interface");
        assert_eq!(lo.kind, InterfaceKind::Loopback);
        assert!(lo.addresses.iter().any(|a| a == "127.0.0.1"));
        assert!(stats.interfaces.iter().all(|i| i.rx_bps >= 0.0 && i.tx_bps >= 0.0));
        assert!(stats.interfaces.iter().any(|i| i.up && i.kind != InterfaceKind::Loopback));
    }
}
