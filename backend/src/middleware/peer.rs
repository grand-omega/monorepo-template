use http::HeaderMap;
use ipnetwork::IpNetwork;
use std::net::IpAddr;

/// Resolve the real client IP, honoring forwarding headers only when the
/// immediate peer is in `trusted_cidrs`.
///
/// Algorithm:
/// - If the TCP peer is not in the trust list, return it and ignore all
///   forwarding headers (an untrusted peer can fabricate them).
/// - Otherwise walk `X-Forwarded-For` right-to-left, skipping any hop that is
///   itself in the trust list, and return the first untrusted hop.
/// - Fall back to `X-Real-IP`, then to the peer.
pub fn client_ip(headers: &HeaderMap, peer: IpAddr, trusted_cidrs: &[IpNetwork]) -> IpAddr {
    if !is_trusted(peer, trusted_cidrs) {
        return peer;
    }

    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        for hop in xff.rsplit(',').map(str::trim) {
            if let Ok(ip) = hop.parse::<IpAddr>()
                && !is_trusted(ip, trusted_cidrs)
            {
                return ip;
            }
        }
    }

    if let Some(real) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && let Ok(ip) = real.trim().parse::<IpAddr>()
        && !is_trusted(ip, trusted_cidrs)
    {
        return ip;
    }

    peer
}

fn is_trusted(ip: IpAddr, cidrs: &[IpNetwork]) -> bool {
    cidrs.iter().any(|n| n.contains(ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    fn cidrs(list: &[&str]) -> Vec<IpNetwork> {
        list.iter().map(|s| s.parse().unwrap()).collect()
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                http::HeaderName::from_static(k),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn untrusted_peer_returns_peer_ignores_xff() {
        let trusted = cidrs(&["10.0.0.0/8"]);
        let peer: IpAddr = "203.0.113.7".parse().unwrap();
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        assert_eq!(client_ip(&h, peer, &trusted), peer);
    }

    #[test]
    fn trusted_peer_uses_xff_first_untrusted_from_right() {
        let trusted = cidrs(&["10.0.0.0/8", "172.16.0.0/12"]);
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        // chain: client → edge(172.16.x) → ingress(10.0.x) → us
        let h = headers(&[("x-forwarded-for", "198.51.100.9, 172.16.4.4, 10.0.0.5")]);
        assert_eq!(
            client_ip(&h, peer, &trusted),
            "198.51.100.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_falls_back_to_real_ip() {
        let trusted = cidrs(&["10.0.0.0/8"]);
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        let h = headers(&[("x-real-ip", "198.51.100.9")]);
        assert_eq!(
            client_ip(&h, peer, &trusted),
            "198.51.100.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_all_hops_trusted_returns_peer() {
        let trusted = cidrs(&["10.0.0.0/8"]);
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        let h = headers(&[("x-forwarded-for", "10.0.0.2, 10.0.0.3")]);
        assert_eq!(client_ip(&h, peer, &trusted), peer);
    }

    #[test]
    fn malformed_xff_falls_back_to_peer() {
        let trusted = cidrs(&["10.0.0.0/8"]);
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        let h = headers(&[("x-forwarded-for", "not-an-ip")]);
        assert_eq!(client_ip(&h, peer, &trusted), peer);
    }
}
