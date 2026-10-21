//! Private crypto mechanism tests grouped by the contract they exercise.

use super::{CryptoKeyError, OwnedSigningKey, SigningOwner};

mod admission;
mod jwt;
mod signing;
mod worker;

fn owner() -> SigningOwner {
    SigningOwner::aws_lc()
}

fn load_pem(pem: &str) -> Result<OwnedSigningKey, CryptoKeyError> {
    owner().load_private_key_pem(pem.as_bytes())
}
