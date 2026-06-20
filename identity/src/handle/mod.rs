use crate::types::HandleResolverOpts;
use anyhow::Result;
use hickory_resolver::config::*;
use hickory_resolver::TokioAsyncResolver;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use url::Url;

pub const SUBDOMAIN: &str = "_atproto";
pub const PREFIX: &str = "did=";

#[derive(Clone, Debug)]
pub struct HandleResolver {
    pub timeout: Duration,
    backup_nameservers: Option<Vec<String>>,
    backup_nameserver_ips: Option<Vec<IpAddr>>,
}

impl HandleResolver {
    pub fn new(opts: HandleResolverOpts) -> Self {
        Self {
            timeout: opts.timeout.unwrap_or(Duration::from_millis(3000)),
            backup_nameservers: opts.backup_nameservers,
            backup_nameserver_ips: None,
        }
    }

    pub async fn resolve(&mut self, handle: &String) -> Result<Option<String>> {
        let dns_future = self.resolve_dns(handle);
        let http_future = self.resolve_http(handle);

        match dns_future.await {
            Ok(dns_res) => Ok(dns_res),
            Err(_) => match http_future.await {
                Ok(http_res) => Ok(http_res),
                Err(_) => self.resolve_backup_dns(handle).await,
            },
        }
    }

    pub async fn resolve_dns(&self, handle: &String) -> Result<Option<String>> {
        let resolver = TokioAsyncResolver::tokio_from_system_conf()?;
        let results = match resolver.txt_lookup(format!("{SUBDOMAIN}.{handle}")).await {
            Ok(res) => res,
            Err(_) => return Ok(None),
        };

        let results = results
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>();

        self.parse_dns_result(results)
    }

    pub async fn resolve_http(&self, handle: &String) -> Result<Option<String>> {
        let url = Url::parse(format!("https://{handle}/.well-known/atproto-did").as_str())?;
        let client = reqwest::Client::new();

        let res = client
            .get(url.as_str())
            .header("Connection", "Keep-Alive")
            .header("Keep-Alive", "timeout=5, max=1000")
            .send()
            .await?;

        let res = res.text().await?;

        let did = match res.split("\n").collect::<Vec<&str>>().first() {
            None => return Ok(None),
            Some(first) => first.trim(),
        };

        match did.starts_with("did:") {
            true => Ok(Some(did.to_string())),
            false => Ok(None),
        }
    }

    pub async fn resolve_backup_dns(&mut self, handle: &String) -> Result<Option<String>> {
        let backup_ips = self.get_backup_nameserver_ips().await?;
        match backup_ips {
            Some(backup_ips) if !backup_ips.is_empty() => {
                let mut config = ResolverConfig::default();
                for ip in backup_ips {
                    config.add_name_server(NameServerConfig {
                        socket_addr: SocketAddr::new(ip, 8080),
                        protocol: Default::default(),
                        tls_dns_name: None,
                        trust_negative_responses: false,
                        bind_addr: None,
                    });
                }

                let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());

                let results = match resolver.txt_lookup(format!("{SUBDOMAIN}.{handle}")).await {
                    Ok(res) => res,
                    Err(_) => return Ok(None),
                };

                let results = results
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>();

                self.parse_dns_result(results)
            }
            _ => Ok(None),
        }
    }

    pub fn parse_dns_result(&self, results: Vec<String>) -> Result<Option<String>> {
        let found = results
            .iter()
            .filter(|i| i.starts_with(PREFIX))
            .collect::<Vec<&String>>();

        match found.len() != 1 {
            true => Ok(None),
            false => Ok(Some(found[0][PREFIX.len()..].to_string())),
        }
    }

    async fn get_backup_nameserver_ips(&mut self) -> Result<Option<Vec<IpAddr>>> {
        match &self.backup_nameservers {
            None => return Ok(None),
            Some(backup_nameservers) => {
                if self.backup_nameserver_ips.is_none() {
                    let resolver = TokioAsyncResolver::tokio_from_system_conf()?;
                    let mut responses = Vec::new();
                    for h in backup_nameservers {
                        if let Ok(res) = resolver.lookup_ip(h).await {
                            responses.push(res);
                        }
                    }

                    for response in responses {
                        let mut backup_nameserver_ips = match &self.backup_nameserver_ips {
                            None => vec![],
                            Some(backup_nameserver_ips) => backup_nameserver_ips.clone(),
                        };
                        backup_nameserver_ips.append(&mut response.iter().collect::<Vec<IpAddr>>());
                        self.backup_nameserver_ips = Some(backup_nameserver_ips);
                    }
                }
            }
        }
        Ok(self.backup_nameserver_ips.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dns_result_single_match() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec!["did=did:plc:test123".to_string()];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, Some("did:plc:test123".to_string()));
    }

    #[test]
    fn test_parse_dns_result_no_match() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec!["some=other".to_string()];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_dns_result_multiple_matches() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec![
            "did=did:plc:first".to_string(),
            "did=did:plc:second".to_string(),
        ];
        // Multiple matches should return None (ambiguous)
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_dns_result_empty_list() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results: Vec<String> = vec![];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_dns_result_did_without_prefix() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec!["not-starting-with-prefix".to_string()];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_dns_result_mixed_results() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec![
            "some=other".to_string(),
            "did=did:plc:the-one".to_string(),
            "another=record".to_string(),
        ];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, Some("did:plc:the-one".to_string()));
    }

    #[test]
    fn test_parse_dns_result_empty_string_entries() {
        let resolver = HandleResolver {
            timeout: Duration::from_millis(3000),
            backup_nameservers: None,
            backup_nameserver_ips: None,
        };
        let results = vec!["".to_string(), "did=did:plc:valid".to_string()];
        let result = resolver.parse_dns_result(results).unwrap();
        assert_eq!(result, Some("did:plc:valid".to_string()));
    }
}
