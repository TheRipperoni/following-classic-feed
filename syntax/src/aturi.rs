use anyhow::{bail, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;

pub fn atp_uri_regex(input: &str) -> Option<Vec<&str>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?i)^(at://)?((?:did:[a-z0-9:%-]+)|(?:[a-z0-9][a-z0-9.:-]*))(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap();
    }
    RE.captures(input).map(|captures| {
        captures
            .iter()
            .skip(1) // Skip the first capture which is the entire match
            .map(|c| c.map_or("", |m| m.as_str()))
            .collect()
    })
}

pub fn relative_regex(input: &str) -> Option<Vec<&str>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?i)^(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap();
    }
    RE.captures(input).map(|captures| {
        captures
            .iter()
            .skip(1) // Skip the first capture which is the entire match
            .map(|c| c.map_or("", |m| m.as_str()))
            .collect()
    })
}

pub struct ParsedOutput {
    pub hash: String,
    pub host: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

pub struct ParsedRelativeOutput {
    pub hash: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AtUri {
    pub hash: String,
    pub host: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

impl AtUri {
    pub fn new(uri: String, base: Option<String>) -> Result<Self> {
        let parsed: ParsedOutput = match base {
            Some(base) => match parse(&base)? {
                None => bail!("Invalid at uri: `{base}`"),
                Some(parsed_base) => match parse_relative(&uri)? {
                    None => bail!("Invalid path: `{uri}`"),
                    Some(relativep) => ParsedOutput {
                        hash: relativep.hash,
                        host: parsed_base.host,
                        pathname: relativep.pathname,
                        search_params: relativep.search_params,
                    },
                },
            },
            None => match parse(&uri)? {
                None => bail!("Invalid at uri: `{uri}`"),
                Some(result) => result,
            },
        };
        Ok(Self {
            hash: parsed.hash,
            host: parsed.host,
            pathname: parsed.pathname,
            search_params: parsed.search_params,
        })
    }

    pub fn make(
        handle_or_did: String,
        collection: Option<String>,
        rkey: Option<String>,
    ) -> Result<Self> {
        let mut str = handle_or_did;
        if let Some(collection) = collection {
            str += format!("/{collection}").as_str();
        }
        if let Some(rkey) = rkey {
            str += format!("/{rkey}").as_str();
        }
        AtUri::new(str, None)
    }

    pub fn get_protocol(&self) -> String {
        "at:".to_string()
    }

    pub fn get_origin(&self) -> String {
        format!("at://{}", self.host)
    }

    pub fn get_hostname(&self) -> &String {
        &self.host
    }

    pub fn set_hostname(&mut self, v: String) {
        self.host = v;
    }

    pub fn get_search(&self) -> Result<Option<String>> {
        let url = Url::parse_with_params("http://example.com", &self.search_params)?;
        match url.query() {
            Some(query) => Ok(Some(query.to_string())),
            None => Ok(None),
        }
    }

    pub fn set_search(&mut self, v: String) -> Result<()> {
        let dummy_url = format!("http://example.com{}", v);
        let url = Url::parse(&dummy_url)?;
        let query_pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|pair| (pair.0.to_string(), pair.1.to_string()))
            .collect();
        self.search_params = query_pairs;
        Ok(())
    }

    pub fn get_collection(&self) -> String {
        self.pathname
            .split("/")
            .find(|s| !s.is_empty())
            .unwrap_or("")
            .to_string()
    }

    pub fn set_collection(&mut self, v: String) {
        let mut parts: Vec<String> = self
            .pathname
            .split("/")
            .filter(|s| !s.is_empty())
            .map(|p| p.to_string())
            .collect();
        if !parts.is_empty() {
            parts[0] = v;
        } else {
            parts.push(v);
        }
        self.pathname = format!("/{}", parts.join("/"));
    }

    pub fn get_rkey(&self) -> String {
        self.pathname
            .split("/")
            .filter(|s| !s.is_empty())
            .nth(1)
            .unwrap_or("")
            .to_string()
    }

    pub fn set_rkey(&mut self, v: String) {
        let mut parts: Vec<String> = self
            .pathname
            .split("/")
            .filter(|s| !s.is_empty())
            .map(|p| p.to_string())
            .collect();
        if parts.len() > 1 {
            parts[1] = v;
        } else if !parts.is_empty() {
            parts.push(v);
        } else {
            parts.push("undefined".to_string());
            parts.push(v);
        }
        self.pathname = format!("/{}", parts.join("/"));
    }

    pub fn get_href(&self) -> String {
        self.to_string()
    }

    pub fn to_string(&self) -> String {
        let mut path = match self.pathname.is_empty() {
            true => "/".to_string(),
            false => self.pathname.clone(),
        };
        if !path.starts_with("/") {
            path = format!("/{path}");
        }
        let qs = match self.get_search() {
            Ok(Some(search_params))
                if !search_params.starts_with("?") && !search_params.is_empty() =>
            {
                format!("?{search_params}")
            }
            Ok(Some(search_params)) => search_params,
            _ => "".to_string(),
        };
        let hash = match self.hash.is_empty() {
            true => self.hash.clone(),
            false => format!("#{}", self.hash),
        };
        format!("at://{}{}{}{}", self.host, path, qs, hash)
    }
}

pub fn parse(str: &str) -> Result<Option<ParsedOutput>> {
    match atp_uri_regex(str) {
        None => Ok(None),
        Some(matches) => {
            // The query string we want to parse
            // e.g. `?q=URLUtils.searchParams&topic=api`
            let query_string = matches[3];
            // Create a dummy base URL and append the query string
            let dummy_url = format!("http://example.com{}", query_string);
            // Parse the URL
            let url = Url::parse(&dummy_url)?;
            let query_pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|pair| (pair.0.to_string(), pair.1.to_string()))
                .collect();
            Ok(Some(ParsedOutput {
                hash: matches[4].trim_start_matches('#').to_string(),
                host: matches[1].to_string(),
                pathname: matches[2].to_string(),
                search_params: query_pairs,
            }))
        }
    }
}

pub fn parse_relative(str: &str) -> Result<Option<ParsedRelativeOutput>> {
    match relative_regex(str) {
        None => Ok(None),
        Some(matches) => {
            // The query string we want to parse
            // e.g. `?q=URLUtils.searchParams&topic=api`
            let query_string = matches[1];
            // Create a dummy base URL and append the query string
            let dummy_url = format!("http://example.com{}", query_string);
            // Parse the URL
            let url = Url::parse(&dummy_url)?;
            let query_pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|pair| (pair.0.to_string(), pair.1.to_string()))
                .collect();
            Ok(Some(ParsedRelativeOutput {
                hash: matches[2].trim_start_matches('#').to_string(),
                pathname: matches[0].to_string(),
                search_params: query_pairs,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_uri_parsing() {
        let uri = AtUri::new("at://bob.com/com.example.post/123".to_string(), None).unwrap();
        assert_eq!(uri.host, "bob.com");
        assert_eq!(uri.pathname, "/com.example.post/123");
        assert_eq!(uri.get_collection(), "com.example.post");
        assert_eq!(uri.get_rkey(), "123");
    }

    #[test]
    fn test_at_uri_make() {
        let uri = AtUri::make(
            "bob.com".to_string(),
            Some("com.example.post".to_string()),
            Some("123".to_string()),
        )
        .unwrap();
        assert_eq!(uri.to_string(), "at://bob.com/com.example.post/123");
    }

    #[test]
    fn test_at_uri_setters() {
        let mut uri = AtUri::new("at://bob.com".to_string(), None).unwrap();
        uri.set_collection("com.example.post".to_string());
        assert_eq!(uri.get_collection(), "com.example.post");
        uri.set_rkey("123".to_string());
        assert_eq!(uri.get_rkey(), "123");
        assert_eq!(uri.to_string(), "at://bob.com/com.example.post/123");
    }

    #[test]
    fn test_at_uri_with_query_and_hash() {
        let uri = AtUri::new(
            "at://bob.com/com.example.post/123?foo=bar#baz".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(uri.host, "bob.com");
        assert_eq!(uri.pathname, "/com.example.post/123");
        assert_eq!(uri.get_search().unwrap().unwrap(), "foo=bar");
        assert_eq!(uri.hash, "baz");
        assert_eq!(
            uri.to_string(),
            "at://bob.com/com.example.post/123?foo=bar#baz"
        );
    }

    #[test]
    fn test_at_uri_make_no_collection() {
        let uri = AtUri::make("bob.com".to_string(), None, None).unwrap();
        assert_eq!(uri.to_string(), "at://bob.com/");
    }
}
