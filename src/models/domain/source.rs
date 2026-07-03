use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SourceHash(String);

impl SourceHash {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(lower_hex(Sha256::digest(bytes).as_ref()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::SourceHash;

    #[test]
    fn source_hash_is_sha256_hex() {
        let source = SourceHash::from_bytes(b"abc");

        assert_eq!(
            source.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
