use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SourceHash(String);

impl SourceHash {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(lower_hex(Sha256::digest(bytes).as_ref()))
    }

    /// Parses a value already claiming to be a source hash (e.g. a path
    /// parameter), validating it's a well-formed lowercase sha256 hex
    /// digest rather than trusting it outright.
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let is_valid = value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());

        is_valid.then_some(Self(value))
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

    #[test]
    fn parse_accepts_a_well_formed_digest_and_rejects_others() {
        let digest = SourceHash::from_bytes(b"abc").as_str().to_owned();

        assert!(SourceHash::parse(digest.clone()).is_some());
        assert!(SourceHash::parse(digest.to_uppercase()).is_none());
        assert!(SourceHash::parse(&digest[..63]).is_none());
        assert!(SourceHash::parse("not-a-hash".repeat(7)).is_none());
    }
}
