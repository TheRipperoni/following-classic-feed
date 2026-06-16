use anyhow::Result;
use multibase::decode;

pub fn multibase_to_bytes(mb: String) -> Result<Vec<u8>> {
    let (_base, data) = decode(mb)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multibase_to_bytes_base58btc() {
        // 'z' prefix = base58btc
        let encoded = multibase::encode(multibase::Base::Base58Btc, vec![0x01, 0x02]);
        let result = multibase_to_bytes(encoded.clone()).unwrap();
        assert_eq!(result, vec![0x01, 0x02]);
    }

    #[test]
    fn test_multibase_to_bytes_base16() {
        // 'f' prefix = base16 (hex)
        let result = multibase_to_bytes("f001122".to_string()).unwrap();
        assert_eq!(result, vec![0x00, 0x11, 0x22]);
    }

    #[test]
    fn test_multibase_to_bytes_invalid() {
        let result = multibase_to_bytes("!!!invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_multibase_to_bytes_empty() {
        let result = multibase_to_bytes("".to_string());
        assert!(result.is_err());
    }
}
