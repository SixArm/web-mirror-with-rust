use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// True if `ip` is a public internet address: not loopback, private,
/// link-local (including cloud metadata, 169.254.169.254), carrier-grade
/// NAT, unspecified, multicast, broadcast, documentation, benchmarking, or
/// reserved. Used by `--check-external`, which requests URLs that pages link
/// to, so a page cannot steer requests at the local network.
pub fn from_ip_addr_into_is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => is_public_v6(v6),
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || a == 0                                  // "this network"
        || (a == 100 && (64..128).contains(&b))   // carrier-grade NAT
        || (a == 192 && b == 0 && c == 0)         // IETF protocol assignments
        || (a == 198 && (b == 18 || b == 19))     // benchmarking
        || a >= 240) // reserved
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00   // unique local
        || (first & 0xffc0) == 0xfe80   // link-local
        || first == 0x2001 && ip.segments()[1] == 0x0db8 // documentation
        || first == 0x0064 && ip.segments()[1] == 0xff9b) // NAT64, which reaches IPv4 inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public(s: &str) -> bool {
        from_ip_addr_into_is_public(s.parse().unwrap())
    }

    #[test]
    fn private_and_special_addresses_are_not_public() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "0.0.0.0",
            "100.64.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "198.18.0.1",
            "240.0.0.1",
            "::1",
            "::",
            "fc00::1",
            "fd12::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "64:ff9b::a00:1",
        ] {
            assert!(!public(ip), "{ip}");
        }
    }

    #[test]
    fn public_addresses_are_public() {
        for ip in ["93.184.216.34", "8.8.8.8", "2606:4700::1111", "::ffff:8.8.8.8"] {
            assert!(public(ip), "{ip}");
        }
    }
}
