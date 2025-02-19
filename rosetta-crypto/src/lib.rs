//! Implements cryptography needed for various chains.
#![deny(missing_docs)]
#![deny(warnings)]

use anyhow::{Context, Result};
use ecdsa::hazmat::SignPrimitive;
use ecdsa::signature::hazmat::PrehashSigner;
use ecdsa::signature::{Signer as _, Verifier as _};
use ecdsa::RecoveryId;
use ed25519_dalek::{Signer as _, Verifier as _};
use sha2::Digest;

pub mod address;
pub mod bip32;
pub use bip39;
pub mod bip44;

/// Signing algorithm.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Algorithm {
    /// ECDSA with secp256k1.
    EcdsaSecp256k1,
    /// ECDSA with secp256k1 in Ethereum compatible format.
    EcdsaRecoverableSecp256k1,
    /// ECDSA with NIST P-256.
    EcdsaSecp256r1,
    /// Ed25519.
    Ed25519,
    /// Schnorrkel used by substrate/polkadot.
    Sr25519,
}

impl Algorithm {
    /// Returns true if the signer's public key is recoverable from the signature.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Algorithm::EcdsaRecoverableSecp256k1)
    }
}

/// Secret key used for constructing signatures.
pub enum SecretKey {
    /// ECDSA with secp256k1.
    EcdsaSecp256k1(ecdsa::SigningKey<k256::Secp256k1>),
    /// ECDSA with secp256k1 in Ethereum compatible format.
    EcdsaRecoverableSecp256k1(ecdsa::SigningKey<k256::Secp256k1>),
    /// ECDSA with NIST P-256.
    EcdsaSecp256r1(ecdsa::SigningKey<p256::NistP256>),
    /// Ed25519.
    Ed25519(ed25519_dalek::Keypair),
    /// Schnorrkel used by substrate/polkadot.
    Sr25519(schnorrkel::Keypair, Option<schnorrkel::MiniSecretKey>),
}

impl Clone for SecretKey {
    fn clone(&self) -> Self {
        Self::from_bytes(self.algorithm(), &self.to_bytes()).unwrap()
    }
}

impl SecretKey {
    /// Returns the signing algorithm.
    pub fn algorithm(&self) -> Algorithm {
        match self {
            SecretKey::EcdsaSecp256k1(_) => Algorithm::EcdsaSecp256k1,
            SecretKey::EcdsaRecoverableSecp256k1(_) => Algorithm::EcdsaRecoverableSecp256k1,
            SecretKey::EcdsaSecp256r1(_) => Algorithm::EcdsaSecp256r1,
            SecretKey::Ed25519(_) => Algorithm::Ed25519,
            SecretKey::Sr25519(_, _) => Algorithm::Sr25519,
        }
    }

    /// Creates a secret key from a byte sequence for a given signing algorithm.
    pub fn from_bytes(algorithm: Algorithm, bytes: &[u8]) -> Result<Self> {
        Ok(match algorithm {
            Algorithm::EcdsaSecp256k1 => {
                SecretKey::EcdsaSecp256k1(ecdsa::SigningKey::from_bytes(bytes.try_into()?)?)
            }
            Algorithm::EcdsaRecoverableSecp256k1 => SecretKey::EcdsaRecoverableSecp256k1(
                ecdsa::SigningKey::from_bytes(bytes.try_into()?)?,
            ),
            Algorithm::EcdsaSecp256r1 => {
                SecretKey::EcdsaSecp256r1(ecdsa::SigningKey::from_bytes(bytes.try_into()?)?)
            }
            Algorithm::Ed25519 => {
                let secret = ed25519_dalek::SecretKey::from_bytes(bytes)?;
                let public = ed25519_dalek::PublicKey::from(&secret);
                let keypair = ed25519_dalek::Keypair { secret, public };
                SecretKey::Ed25519(keypair)
            }
            Algorithm::Sr25519 => {
                if bytes.len() == 32 {
                    let minisecret = schnorrkel::MiniSecretKey::from_bytes(bytes)
                        .map_err(|err| anyhow::anyhow!("{}", err))?;
                    let secret =
                        minisecret.expand_to_keypair(schnorrkel::MiniSecretKey::ED25519_MODE);
                    SecretKey::Sr25519(secret, Some(minisecret))
                } else {
                    let secret = schnorrkel::SecretKey::from_bytes(bytes)
                        .map_err(|err| anyhow::anyhow!("{}", err))?;
                    SecretKey::Sr25519(secret.to_keypair(), None)
                }
            }
        })
    }

    /// Returns a byte sequence representing the secret key.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SecretKey::EcdsaSecp256k1(secret) => secret.to_bytes().to_vec(),
            SecretKey::EcdsaRecoverableSecp256k1(secret) => secret.to_bytes().to_vec(),
            SecretKey::EcdsaSecp256r1(secret) => secret.to_bytes().to_vec(),
            SecretKey::Ed25519(secret) => secret.secret.to_bytes().to_vec(),
            SecretKey::Sr25519(_, Some(minisecret)) => minisecret.as_bytes().to_vec(),
            SecretKey::Sr25519(secret, None) => secret.secret.to_bytes().to_vec(),
        }
    }

    /// Returns the public key used for verifying signatures.
    pub fn public_key(&self) -> PublicKey {
        match self {
            SecretKey::EcdsaSecp256k1(secret) => PublicKey::EcdsaSecp256k1(*secret.verifying_key()),
            SecretKey::EcdsaRecoverableSecp256k1(secret) => {
                PublicKey::EcdsaRecoverableSecp256k1(*secret.verifying_key())
            }
            SecretKey::EcdsaSecp256r1(secret) => PublicKey::EcdsaSecp256r1(*secret.verifying_key()),
            SecretKey::Ed25519(secret) => PublicKey::Ed25519(secret.public),
            SecretKey::Sr25519(secret, _) => PublicKey::Sr25519(secret.public),
        }
    }

    /// Signs a message and returns it's signature.
    pub fn sign(&self, msg: &[u8], context_param: &str) -> Signature {
        match self {
            SecretKey::EcdsaSecp256k1(secret) => Signature::EcdsaSecp256k1(secret.sign(msg)),
            SecretKey::EcdsaRecoverableSecp256k1(_) => {
                let digest = sha2::Sha256::digest(msg);
                self.sign_prehashed(&digest).unwrap()
            }
            SecretKey::EcdsaSecp256r1(secret) => Signature::EcdsaSecp256r1(secret.sign(msg)),
            SecretKey::Ed25519(secret) => Signature::Ed25519(secret.sign(msg)),
            SecretKey::Sr25519(secret, _) => {
                // need a signing context here for substrate
                let context = schnorrkel::signing_context(context_param.as_bytes());
                Signature::Sr25519(secret.sign(context.bytes(msg)))
            }
        }
    }

    /// Signs a prehashed message and returns it's signature.
    pub fn sign_prehashed(&self, hash: &[u8]) -> Result<Signature> {
        Ok(match self {
            SecretKey::EcdsaSecp256k1(secret) => {
                Signature::EcdsaSecp256k1(secret.sign_prehash(hash)?)
            }
            SecretKey::EcdsaRecoverableSecp256k1(secret) => {
                let (sig, recid) = secret
                    .as_nonzero_scalar()
                    .try_sign_prehashed_rfc6979::<sha2::Sha256>(hash.try_into()?, b"")?;
                Signature::EcdsaRecoverableSecp256k1(sig, recid.context("no recovery id")?)
            }
            SecretKey::EcdsaSecp256r1(secret) => {
                Signature::EcdsaSecp256r1(secret.sign_prehash(hash)?)
            }
            SecretKey::Ed25519(_) => anyhow::bail!("unimplemented"),
            SecretKey::Sr25519(_, _) => {
                anyhow::bail!("unsupported")
            }
        })
    }
}

/// Public key used for verifying signatures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicKey {
    /// ECDSA with secp256k1.
    EcdsaSecp256k1(ecdsa::VerifyingKey<k256::Secp256k1>),
    /// ECDSA with secp256k1 in Ethereum compatible format.
    EcdsaRecoverableSecp256k1(ecdsa::VerifyingKey<k256::Secp256k1>),
    /// ECDSA with NIST P-256.
    EcdsaSecp256r1(ecdsa::VerifyingKey<p256::NistP256>),
    /// Ed25519.
    Ed25519(ed25519_dalek::PublicKey),
    /// Schnorrkel used by substrate/polkadot.
    Sr25519(schnorrkel::PublicKey),
}

impl PublicKey {
    /// Returns the signing algorithm.
    pub fn algorithm(&self) -> Algorithm {
        match self {
            PublicKey::EcdsaSecp256k1(_) => Algorithm::EcdsaSecp256k1,
            PublicKey::EcdsaRecoverableSecp256k1(_) => Algorithm::EcdsaRecoverableSecp256k1,
            PublicKey::EcdsaSecp256r1(_) => Algorithm::EcdsaSecp256r1,
            PublicKey::Ed25519(_) => Algorithm::Ed25519,
            PublicKey::Sr25519(_) => Algorithm::Sr25519,
        }
    }

    /// Creates a public key from a byte sequence for a given signing algorithm.
    pub fn from_bytes(algorithm: Algorithm, bytes: &[u8]) -> Result<Self> {
        Ok(match algorithm {
            Algorithm::EcdsaSecp256k1 => {
                PublicKey::EcdsaSecp256k1(ecdsa::VerifyingKey::from_sec1_bytes(bytes)?)
            }
            Algorithm::EcdsaRecoverableSecp256k1 => {
                PublicKey::EcdsaRecoverableSecp256k1(ecdsa::VerifyingKey::from_sec1_bytes(bytes)?)
            }
            Algorithm::EcdsaSecp256r1 => {
                PublicKey::EcdsaSecp256r1(ecdsa::VerifyingKey::from_sec1_bytes(bytes)?)
            }
            Algorithm::Ed25519 => PublicKey::Ed25519(ed25519_dalek::PublicKey::from_bytes(bytes)?),
            Algorithm::Sr25519 => {
                let public = schnorrkel::PublicKey::from_bytes(bytes)
                    .map_err(|err| anyhow::anyhow!("{}", err))?;
                PublicKey::Sr25519(public)
            }
        })
    }

    /// Returns a byte sequence representing the public key.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::EcdsaSecp256k1(public) => public.to_encoded_point(true).as_bytes().to_vec(),
            PublicKey::EcdsaRecoverableSecp256k1(public) => {
                public.to_encoded_point(true).as_bytes().to_vec()
            }
            PublicKey::EcdsaSecp256r1(public) => public.to_encoded_point(true).as_bytes().to_vec(),
            PublicKey::Ed25519(public) => public.to_bytes().to_vec(),
            PublicKey::Sr25519(public) => public.to_bytes().to_vec(),
        }
    }

    /// Returns an uncompressed byte sequence representing the public key.
    pub fn to_uncompressed_bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::EcdsaSecp256k1(public) => public.to_encoded_point(false).as_bytes().to_vec(),
            PublicKey::EcdsaRecoverableSecp256k1(public) => {
                public.to_encoded_point(false).as_bytes().to_vec()
            }
            PublicKey::EcdsaSecp256r1(public) => public.to_encoded_point(false).as_bytes().to_vec(),
            PublicKey::Ed25519(public) => public.to_bytes().to_vec(),
            PublicKey::Sr25519(public) => public.to_bytes().to_vec(),
        }
    }

    /// Verifies a signature.
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> Result<()> {
        match (self, &sig) {
            (PublicKey::EcdsaSecp256k1(public), Signature::EcdsaSecp256k1(sig)) => {
                public.verify(msg, sig)?
            }
            (
                PublicKey::EcdsaRecoverableSecp256k1(public),
                Signature::EcdsaRecoverableSecp256k1(sig, _),
            ) => public.verify(msg, sig)?,
            (PublicKey::EcdsaSecp256r1(public), Signature::EcdsaSecp256r1(sig)) => {
                public.verify(msg, sig)?
            }
            (PublicKey::Ed25519(public), Signature::Ed25519(sig)) => public.verify(msg, sig)?,
            (PublicKey::Sr25519(public), Signature::Sr25519(sig)) => {
                public
                    .verify_simple(&[], msg, sig)
                    .map_err(|err| anyhow::anyhow!("{}", err))?;
            }
            (_, _) => anyhow::bail!("unsupported signature scheme"),
        };
        Ok(())
    }
}

/// Signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signature {
    /// ECDSA with secp256k1.
    EcdsaSecp256k1(ecdsa::Signature<k256::Secp256k1>),
    /// ECDSA with secp256k1 in Ethereum compatible format.
    EcdsaRecoverableSecp256k1(ecdsa::Signature<k256::Secp256k1>, RecoveryId),
    /// ECDSA with NIST P-256.
    EcdsaSecp256r1(ecdsa::Signature<p256::NistP256>),
    /// Ed25519.
    Ed25519(ed25519_dalek::Signature),
    /// Schnorrkel used by substrate/polkadot.
    Sr25519(schnorrkel::Signature),
}

impl Signature {
    /// Returns the signing algorithm.
    pub fn algorithm(&self) -> Algorithm {
        match self {
            Signature::EcdsaSecp256k1(_) => Algorithm::EcdsaSecp256k1,
            Signature::EcdsaRecoverableSecp256k1(_, _) => Algorithm::EcdsaRecoverableSecp256k1,
            Signature::EcdsaSecp256r1(_) => Algorithm::EcdsaSecp256r1,
            Signature::Ed25519(_) => Algorithm::Ed25519,
            Signature::Sr25519(_) => Algorithm::Sr25519,
        }
    }

    /// Creates a signature from a byte sequence for a given signing algorithm.
    pub fn from_bytes(algorithm: Algorithm, bytes: &[u8]) -> Result<Self> {
        Ok(match algorithm {
            Algorithm::EcdsaSecp256k1 => {
                Signature::EcdsaSecp256k1(ecdsa::Signature::try_from(bytes)?)
            }
            Algorithm::EcdsaRecoverableSecp256k1 => Signature::EcdsaRecoverableSecp256k1(
                ecdsa::Signature::try_from(&bytes[..64])?,
                RecoveryId::from_byte(bytes[64]).context("invalid signature")?,
            ),
            Algorithm::EcdsaSecp256r1 => {
                Signature::EcdsaSecp256r1(ecdsa::Signature::try_from(bytes)?)
            }
            Algorithm::Ed25519 => Signature::Ed25519(ed25519_dalek::Signature::from_bytes(bytes)?),
            Algorithm::Sr25519 => {
                let sig = schnorrkel::Signature::from_bytes(bytes)
                    .map_err(|err| anyhow::anyhow!("{}", err))?;
                Signature::Sr25519(sig)
            }
        })
    }

    /// Returns a byte sequence representing the signature.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Signature::EcdsaSecp256k1(sig) => sig.to_vec(),
            Signature::EcdsaRecoverableSecp256k1(sig, recovery_id) => {
                let mut bytes = Vec::with_capacity(65);
                bytes.extend(sig.to_bytes());
                bytes.push(recovery_id.to_byte());
                bytes
            }
            Signature::EcdsaSecp256r1(sig) => sig.to_vec(),
            Signature::Ed25519(sig) => sig.to_bytes().to_vec(),
            Signature::Sr25519(sig) => sig.to_bytes().to_vec(),
        }
    }

    /// Returns the recovered public key if supported.
    pub fn recover(&self, msg: &[u8]) -> Result<Option<PublicKey>> {
        if let Signature::EcdsaRecoverableSecp256k1(signature, recovery_id) = self {
            let recovered_key =
                ecdsa::VerifyingKey::recover_from_msg(msg, signature, *recovery_id)?;
            Ok(Some(PublicKey::EcdsaRecoverableSecp256k1(recovered_key)))
        } else {
            Ok(None)
        }
    }

    /// Returns the recovered public key if supported.
    pub fn recover_prehashed(&self, hash: &[u8]) -> Result<Option<PublicKey>> {
        if let Signature::EcdsaRecoverableSecp256k1(signature, recovery_id) = self {
            let recovered_key =
                ecdsa::VerifyingKey::recover_from_prehash(hash, signature, *recovery_id)?;
            Ok(Some(PublicKey::EcdsaRecoverableSecp256k1(recovered_key)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, RngCore};

    const ALGORITHMS: &[Algorithm] = &[
        Algorithm::EcdsaSecp256k1,
        Algorithm::EcdsaRecoverableSecp256k1,
        Algorithm::EcdsaSecp256r1,
        Algorithm::Ed25519,
        Algorithm::Sr25519,
    ];

    #[test]
    fn secret_key_bytes() -> Result<()> {
        let mut rng = thread_rng();
        let mut secret = [0; 32];
        rng.fill_bytes(&mut secret);
        for curve in ALGORITHMS {
            let secret_key = SecretKey::from_bytes(*curve, &secret[..])?;
            let secret2 = secret_key.to_bytes();
            assert_eq!(&secret[..], secret2);
        }
        Ok(())
    }

    #[test]
    fn public_key_bytes() -> Result<()> {
        let mut rng = thread_rng();
        let mut secret = [0; 32];
        rng.fill_bytes(&mut secret);
        for algorithm in ALGORITHMS {
            let secret_key = SecretKey::from_bytes(*algorithm, &secret[..])?;
            let public_key = secret_key.public_key();
            let public = public_key.to_bytes();
            let public_key2 = PublicKey::from_bytes(*algorithm, &public)?;
            assert_eq!(public_key, public_key2);
        }
        Ok(())
    }

    #[test]
    fn signature_bytes() -> Result<()> {
        let mut rng = thread_rng();
        let mut secret = [0; 32];
        rng.fill_bytes(&mut secret);
        let mut msg = [0; 32];
        rng.fill_bytes(&mut msg);
        for algorithm in ALGORITHMS {
            let secret_key = SecretKey::from_bytes(*algorithm, &secret[..])?;
            let signature = secret_key.sign(&msg, "");
            let sig = signature.to_bytes();
            let signature2 = Signature::from_bytes(*algorithm, &sig[..])?;
            assert_eq!(signature, signature2);
        }
        Ok(())
    }

    #[test]
    fn sign_verify() -> Result<()> {
        let mut rng = thread_rng();
        let mut secret = [0; 32];
        rng.fill_bytes(&mut secret);
        let mut msg = [0; 32];
        rng.fill_bytes(&mut msg);
        for algorithm in ALGORITHMS {
            let secret_key = SecretKey::from_bytes(*algorithm, &secret[..])?;
            let public_key = secret_key.public_key();
            let signature = secret_key.sign(&msg, "");
            public_key.verify(&msg, &signature)?;
        }
        Ok(())
    }

    #[test]
    fn sign_recover_pubkey() -> Result<()> {
        let mut rng = thread_rng();
        let mut secret = [0; 32];
        rng.fill_bytes(&mut secret);
        let mut msg = [0; 32];
        rng.fill_bytes(&mut msg);
        let secret_key = SecretKey::from_bytes(Algorithm::EcdsaRecoverableSecp256k1, &secret[..])?;
        let public_key = secret_key.public_key();
        let signature = secret_key.sign(&msg, "");
        let recovered_key = signature.recover(&msg)?.unwrap();
        assert_eq!(public_key, recovered_key);
        Ok(())
    }
}
