use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;

pub fn atp_uri_regex(input: &str) -> Option<Vec<&str>> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^(at://)?((?:did:[a-z0-9:%-]+)|(?:[a-z0-9][a-z0-9.:-]*))(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap()
    });
    RE.captures(input).map(|captures| {
        captures
            .iter()
            .skip(1) // Skip the first capture which is the entire match
            .map(|c| c.map_or("", |m| m.as_str()))
            .collect()
    })
}

pub fn relative_regex(input: &str) -> Option<Vec<&str>> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap()
    });
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

    #[test]
    fn test_at_uri_make_only_collection() {
        let uri = AtUri::make(
            "bob.com".to_string(),
            Some("com.example.post".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(uri.to_string(), "at://bob.com/com.example.post");
    }

    #[test]
    fn test_at_uri_make_only_rkey() {
        let uri = AtUri::make("bob.com".to_string(), None, Some("123".to_string())).unwrap();
        assert_eq!(uri.to_string(), "at://bob.com/123");
    }

    #[test]
    fn test_at_uri_did_hostname() {
        let uri = AtUri::new("at://did:plc:abc/app.bsky.feed.post/123".to_string(), None).unwrap();
        assert_eq!(uri.get_hostname(), "did:plc:abc");
        assert_eq!(uri.get_collection(), "app.bsky.feed.post");
        assert_eq!(uri.get_rkey(), "123");
    }

    #[test]
    fn test_at_uri_with_base() {
        let uri = AtUri::new(
            "/com.example.post/456".to_string(),
            Some("at://bob.com".to_string()),
        )
        .unwrap();
        assert_eq!(uri.host, "bob.com");
        assert_eq!(uri.get_collection(), "com.example.post");
        assert_eq!(uri.get_rkey(), "456");
    }

    #[test]
    fn test_at_uri_invalid() {
        let err = AtUri::new("".to_string(), None);
        assert!(err.is_err());
    }

    #[test]
    fn test_at_uri_invalid_with_hash_only() {
        let err = AtUri::new("#hash-only".to_string(), None);
        assert!(err.is_err());
    }

    #[test]
    fn test_at_uri_invalid_base() {
        let err = AtUri::new("/path".to_string(), Some("".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn test_at_uri_invalid_relative() {
        let err = AtUri::new(
            "not-a-relative-path".to_string(),
            Some("at://bob.com".to_string()),
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_at_uri_protocol_and_origin() {
        let uri = AtUri::new("at://alice.com/com.example.post/1".to_string(), None).unwrap();
        assert_eq!(uri.get_protocol(), "at:");
        assert_eq!(uri.get_origin(), "at://alice.com");
    }

    #[test]
    fn test_at_uri_set_hostname() {
        let mut uri = AtUri::new("at://bob.com/com.example.post/1".to_string(), None).unwrap();
        uri.set_hostname("alice.com".to_string());
        assert_eq!(uri.get_hostname(), "alice.com");
    }

    #[test]
    fn test_at_uri_set_search() {
        let mut uri = AtUri::new("at://bob.com".to_string(), None).unwrap();
        uri.set_search("?foo=bar&baz=qux".to_string()).unwrap();
        assert_eq!(uri.get_search().unwrap().unwrap(), "foo=bar&baz=qux");
    }

    #[test]
    fn test_at_uri_get_href() {
        let uri = AtUri::new("at://bob.com/com.example.post/1".to_string(), None).unwrap();
        assert_eq!(uri.get_href(), uri.to_string());
    }

    #[test]
    fn test_at_uri_deep_pathname() {
        let mut uri = AtUri::new(
            "at://bob.com/com.example.post/123/more/deep".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(uri.get_collection(), "com.example.post");
        assert_eq!(uri.get_rkey(), "123");

        uri.set_collection("other.collection".to_string());
        assert_eq!(uri.get_collection(), "other.collection");

        uri.set_rkey("999".to_string());
        assert_eq!(uri.get_rkey(), "999");
    }

    #[test]
    fn test_at_uri_rkey_before_collection() {
        let mut uri = AtUri::new("at://bob.com".to_string(), None).unwrap();
        uri.set_rkey("abc".to_string());
        assert_eq!(uri.get_rkey(), "abc");
        assert_eq!(uri.to_string(), "at://bob.com/undefined/abc");
    }

    #[test]
    fn test_parse_invalid_str() {
        let result = parse("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_valid() {
        let result = parse("at://bob.com/com.example.post/123?key=val#hash").unwrap();
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.host, "bob.com");
        assert_eq!(parsed.pathname, "/com.example.post/123");
        assert_eq!(
            parsed.search_params,
            vec![("key".to_string(), "val".to_string())]
        );
        assert_eq!(parsed.hash, "hash");
    }

    #[test]
    fn test_parse_relative_valid() {
        let result = parse_relative("/com.example.post/123?key=val#hash").unwrap();
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.pathname, "/com.example.post/123");
        assert_eq!(parsed.hash, "hash");
    }

    #[test]
    fn test_parse_relative_path_only() {
        let result = parse_relative("/path/to/something").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().pathname, "/path/to/something");
    }

    #[test]
    fn test_parse_relative_empty() {
        let result = parse_relative("").unwrap();
        assert!(result.is_some()); // matches the regex
    }

    #[test]
    fn test_atp_uri_regex_did() {
        let result = atp_uri_regex("at://did:plc:abc123/app.bsky.feed.post/1?a=b#c");
        assert!(result.is_some());
        let parts = result.unwrap();
        assert_eq!(parts[1], "did:plc:abc123");
        assert_eq!(parts[2], "/app.bsky.feed.post/1");
        assert_eq!(parts[3], "?a=b");
        assert_eq!(parts[4], "#c");
    }

    #[test]
    fn test_atp_uri_regex_no_protocol() {
        let result = atp_uri_regex("bob.com");
        assert!(result.is_some());
    }

    #[test]
    fn test_relative_regex_empty() {
        let result = relative_regex("");
        assert!(result.is_some());
    }
}
