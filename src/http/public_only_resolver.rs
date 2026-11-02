use crate::http::from_ip_addr_into_is_public::from_ip_addr_into_is_public;
use ::reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;

/// A DNS resolver for `--check-external` that keeps only public addresses
/// (see `from_ip_addr_into_is_public`). A name that resolves only to private
/// or special addresses fails to connect, so a link to
/// `http://router.home/` or a name pointed at 127.0.0.1 is never requested.
/// Because the check is on the resolved address that is then connected to,
/// a name cannot pass it and then switch to a private address (DNS
/// rebinding).
pub struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|addr| from_ip_addr_into_is_public(addr.ip()))
                .collect();
            if addrs.is_empty() {
                return Err(format!("{host} has no public address").into());
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn refuses_names_of_private_addresses() {
        // `localhost` resolves to loopback only, without the network.
        let result = PublicOnlyResolver.resolve(Name::from_str("localhost").unwrap()).await;
        assert!(result.is_err());
    }
}
