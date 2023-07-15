//! Module containing the logic to generate a Prefixed API Key.
//! Code is heavily borrowed from https://github.com/brahmlower/prefixed-api-key

use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt::{self, Debug},
};

use rand::{rngs::OsRng, RngCore};

#[derive(Debug, PartialEq, Eq)]
pub enum PrefixedApiKeyError {
    WrongNumberOfParts(usize),
}

impl Error for PrefixedApiKeyError {}

impl fmt::Display for PrefixedApiKeyError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{self:?}")
    }
}

pub struct PrefixedApiKey {
    prefix: String,
    short_token: String,
    long_token: String,
}

impl PrefixedApiKey {
    pub fn new(prefix: String, short_token: String, long_token: String) -> Self {
        PrefixedApiKey {
            prefix,
            short_token,
            long_token,
        }
    }

    pub fn prefixed(&self) -> &str {
        &self.prefix
    }

    pub fn short_token(&self) -> &str {
        &self.short_token
    }

    pub fn long_token(&self) -> &str {
        &self.long_token
    }

    pub fn long_token_hashed(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.long_token.clone());
        hex::encode(hasher.finalize_reset())
    }

    pub fn from_string(pak_string: &str) -> Result<PrefixedApiKey, PrefixedApiKeyError> {
        let parts: Vec<&str> = pak_string.split('_').collect();

        if parts.len() != 3 {
            return Err(PrefixedApiKeyError::WrongNumberOfParts(parts.len()));
        }

        Ok(PrefixedApiKey::new(
            parts[0].to_owned(),
            parts[1].to_owned(),
            parts[2].to_owned(),
        ))
    }
}

/// A custom implementation of Debug that masks the secret long token that way
/// the struct can be debug printed without leaking sensitive info into logs
impl Debug for PrefixedApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrefixedApiKey")
            .field("prefix", &self.prefix)
            .field("short_token", &self.short_token)
            .field("long_token", &"***")
            .finish()
    }
}

impl ToString for PrefixedApiKey {
    fn to_string(&self) -> String {
        format!("{}_{}_{}", self.prefix, self.short_token, self.long_token)
    }
}

impl TryInto<PrefixedApiKey> for &str {
    type Error = PrefixedApiKeyError;

    fn try_into(self) -> Result<PrefixedApiKey, Self::Error> {
        PrefixedApiKey::from_string(self)
    }
}

pub struct PrefixedApiKeyController {
    prefix: String,
    short_token_length: usize,
    long_token_length: usize,
}

impl PrefixedApiKeyController {
    pub fn new(prefix: String, short_token_length: usize, long_token_length: usize) -> Self {
        Self {
            prefix,
            short_token_length,
            long_token_length,
        }
    }

    fn get_random_bytes(&self, length: usize) -> Vec<u8> {
        let mut random_bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut random_bytes);
        random_bytes
    }

    fn get_random_token(&self, length: usize) -> String {
        let bytes = self.get_random_bytes(length);
        bs58::encode(bytes).into_string()
    }

    pub fn generate_key(&self) -> PrefixedApiKey {
        let short_token = self.get_random_token(self.short_token_length);
        let long_token = self.get_random_token(self.long_token_length);

        PrefixedApiKey::new(self.prefix.to_owned(), short_token, long_token)
    }

    pub fn long_token_hashed(&self, pak: &PrefixedApiKey) -> String {
        pak.long_token_hashed()
    }

    pub fn generate_key_and_hash(&self) -> (PrefixedApiKey, String) {
        let pak = self.generate_key();
        let hash = self.long_token_hashed(&pak);
        (pak, hash)
    }
}
