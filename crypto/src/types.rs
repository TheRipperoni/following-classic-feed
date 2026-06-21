use anyhow::Result;

pub type VerifySignatureFn = fn(&String, &[u8], &[u8], Option<VerifyOptions>) -> Result<bool>;

pub struct DidKeyPlugin<'p> {
    pub prefix: [u8; 2],
    pub jwt_alg: &'p str,
    pub compress_pubkey: fn(Vec<u8>) -> Result<Vec<u8>>,
    pub decompress_pubkey: fn(Vec<u8>) -> Result<Vec<u8>>,
    pub verify_signature: VerifySignatureFn,
}

pub struct VerifyOptions {
    pub allow_malleable_sig: Option<bool>,
}
