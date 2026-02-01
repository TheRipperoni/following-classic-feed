use anyhow::Result;
use multibase::decode;

pub fn multibase_to_bytes(mb: String) -> Result<Vec<u8>> {
    let (_base, data) = decode(mb)?;
    Ok(data)
}
