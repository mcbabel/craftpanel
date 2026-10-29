use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::config::Proxy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Behind(IpAddr);

pub async fn resolve(
    State(trusted): State<Arc<Vec<Proxy>>>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>().copied()
    {
        let caller = through(&trusted, peer.ip(), request.headers());
        request.extensions_mut().insert(Behind(caller));
    }
    next.run(request).await
}

pub fn caller_address(parts: &Parts) -> Option<IpAddr> {
    if let Some(Behind(address)) = parts.extensions.get::<Behind>() {
        return Some(*address);
    }
    parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|info| info.0.ip().to_canonical())
}

fn through(trusted: &[Proxy], peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    let trusts = |address: IpAddr| trusted.iter().any(|proxy| proxy.covers(address));

    let mut caller = peer.to_canonical();
    if !trusts(caller) {
        return caller;
    }

    let hops: Vec<&str> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .collect();

    for hop in hops.into_iter().rev() {
        let Ok(address) = hop.parse::<IpAddr>() else {
            break;
        };
        caller = address.to_canonical();
        if !trusts(caller) {
            break;
        }
    }
    caller
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn proxies(raw: &[&str]) -> Vec<Proxy> {
        raw.iter().map(|one| Proxy::try_from(one.to_string()).unwrap()).collect()
    }

    fn forwarded(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append("x-forwarded-for", HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    fn ip(raw: &str) -> IpAddr {
        raw.parse().unwrap()
    }

    #[test]
    fn without_trusted_proxies_the_connection_is_the_caller() {
        let headers = forwarded(&["203.0.113.7"]);
        assert_eq!(through(&[], ip("192.168.178.180"), &headers), ip("192.168.178.180"));
    }

    #[test]
    fn a_stranger_cannot_name_another_address() {
        let trusted = proxies(&["192.168.178.180"]);
        let headers = forwarded(&["203.0.113.7"]);
        assert_eq!(through(&trusted, ip("198.51.100.9"), &headers), ip("198.51.100.9"));
    }

    #[test]
    fn behind_two_proxies_the_first_untrusted_hop_from_the_right_is_the_caller() {
        let trusted = proxies(&["192.168.178.180", "10.0.0.1"]);
        let headers = forwarded(&["1.2.3.4, 203.0.113.7, 10.0.0.1"]);
        assert_eq!(through(&trusted, ip("192.168.178.180"), &headers), ip("203.0.113.7"));
    }

    #[test]
    fn hops_spread_over_several_headers_are_read_in_order() {
        let trusted = proxies(&["192.168.178.180", "10.0.0.1"]);
        let headers = forwarded(&["203.0.113.7", "10.0.0.1"]);
        assert_eq!(through(&trusted, ip("192.168.178.180"), &headers), ip("203.0.113.7"));
    }

    #[test]
    fn a_hop_that_is_no_address_stops_the_walk_at_the_last_proxy() {
        let trusted = proxies(&["192.168.178.180", "10.0.0.1"]);
        let headers = forwarded(&["203.0.113.7, unknown, 10.0.0.1"]);
        assert_eq!(through(&trusted, ip("192.168.178.180"), &headers), ip("10.0.0.1"));
    }

    #[test]
    fn a_trusted_proxy_that_forwards_nothing_is_itself_the_caller() {
        let trusted = proxies(&["192.168.178.180"]);
        assert_eq!(
            through(&trusted, ip("192.168.178.180"), &HeaderMap::new()),
            ip("192.168.178.180")
        );
    }

    #[test]
    fn mapped_addresses_count_as_their_plain_form() {
        let trusted = proxies(&["192.168.178.180"]);
        let headers = forwarded(&["::ffff:203.0.113.7"]);
        assert_eq!(through(&trusted, ip("::ffff:192.168.178.180"), &headers), ip("203.0.113.7"));
    }
}
