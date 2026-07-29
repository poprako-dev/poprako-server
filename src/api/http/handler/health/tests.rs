use super::*;

// is_loopback(is_loopback)(positive): IPv4 and IPv6 loopback callers are accepted.
// is_loopback(is_loopback)(negative): non-loopback callers are rejected.

#[test]
fn is_loopback_accepts_only_loopback_addresses() {
    //
    let ipv4_loopback = SocketAddr::from(([127, 0, 0, 1], 8080));

    let ipv6_loopback = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 8080));

    let public_addr = SocketAddr::from(([203, 0, 113, 1], 8080));

    assert!(is_loopback(ipv4_loopback));

    assert!(is_loopback(ipv6_loopback));

    assert!(!is_loopback(public_addr));
}
