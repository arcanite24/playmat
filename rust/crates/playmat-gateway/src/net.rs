//! Client addresses behind proxies. The gateway forwards `X-Forwarded-For` and `X-Real-IP` to Java, which trusts
//! them from the gateway; so the gateway must only pass them on from proxies it trusts itself, and otherwise set
//! them to the TCP peer.

use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct Cidr {
    network: IpAddr,
    bits: u8,
}

impl Cidr {
    pub fn parse(text: &str) -> Result<Cidr, String> {
        let text = text.trim();
        let (address, bits) = match text.split_once('/') {
            Some((address, bits)) => (address, Some(bits.parse::<u8>().map_err(|_| format!("bad prefix in {text}"))?)),
            None => (text, None),
        };
        let network: IpAddr = address.parse().map_err(|_| format!("bad address {text}"))?;
        let max = if network.is_ipv4() { 32 } else { 128 };
        let bits = bits.unwrap_or(max);
        if bits > max {
            return Err(format!("prefix over {max} in {text}"));
        }
        Ok(Cidr { network, bits })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
            v4 => v4,
        };
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = if self.bits == 0 { 0 } else { u32::MAX << (32 - self.bits) };
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = if self.bits == 0 { 0 } else { u128::MAX << (128 - self.bits) };
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

pub fn parse_list(text: &str) -> Result<Vec<Cidr>, String> {
    text.split(',').filter(|s| !s.trim().is_empty()).map(Cidr::parse).collect()
}

/// The client's address: the peer, or, when the peer is a trusted proxy, the right-most address in
/// `X-Forwarded-For` that isn't one (else `X-Real-IP`).
pub fn client_ip(peer: IpAddr, forwarded_for: Option<&str>, real_ip: Option<&str>, trusted: &[Cidr]) -> IpAddr {
    let is_trusted = |ip: IpAddr| trusted.iter().any(|cidr| cidr.contains(ip));
    if !is_trusted(peer) {
        return peer;
    }
    if let Some(chain) = forwarded_for {
        let hops: Vec<IpAddr> = chain.split(',').filter_map(|hop| hop.trim().parse().ok()).collect();
        if let Some(client) = hops.iter().rev().find(|ip| !is_trusted(**ip)) {
            return *client;
        }
        if let Some(first) = hops.first() {
            return *first;
        }
    }
    real_ip.and_then(|ip| ip.trim().parse().ok()).unwrap_or(peer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidrs() {
        let net = Cidr::parse("172.16.0.0/12").unwrap();
        assert!(net.contains("172.20.1.2".parse().unwrap()));
        assert!(!net.contains("172.32.0.1".parse().unwrap()));
        assert!(Cidr::parse("::1").unwrap().contains("::1".parse().unwrap()));
        assert!(Cidr::parse("127.0.0.1").unwrap().contains("::ffff:127.0.0.1".parse().unwrap()));
        assert!(Cidr::parse("1.2.3.4/40").is_err());
    }

    #[test]
    fn forwarded_only_from_trusted_peers() {
        let trusted = parse_list("127.0.0.1,10.0.0.0/8").unwrap();
        let peer: IpAddr = "10.0.0.5".parse().unwrap();
        assert_eq!(client_ip(peer, Some("8.8.8.8, 10.0.0.9"), None, &trusted), "8.8.8.8".parse::<IpAddr>().unwrap());
        assert_eq!(client_ip(peer, None, Some("9.9.9.9"), &trusted), "9.9.9.9".parse::<IpAddr>().unwrap());
        let stranger: IpAddr = "5.5.5.5".parse().unwrap();
        assert_eq!(client_ip(stranger, Some("8.8.8.8"), None, &trusted), stranger);
    }
}
