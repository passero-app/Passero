#![allow(dead_code)]

use std::io::Write;

use sequoia_openpgp::cert::prelude::*;
use sequoia_openpgp::crypto::SessionKey;
use sequoia_openpgp::parse::stream::*;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::policy::{Policy, StandardPolicy};
use sequoia_openpgp::serialize::stream::*;
use sequoia_openpgp::serialize::SerializeInto;
use sequoia_openpgp::types::SymmetricAlgorithm;
pub use sequoia_openpgp::cert::Cert;
use sequoia_openpgp::KeyHandle;

use crate::{CoreError, Result};

fn map_crypto<E: std::fmt::Display>(e: E) -> CoreError {
    CoreError::Crypto(e.to_string())
}

pub fn generate_key(user_id: &str) -> Result<(Cert, Vec<u8>)> {
    let (cert, _revocation) = CertBuilder::new()
        .add_userid(user_id)
        .add_transport_encryption_subkey()
        .generate()
        .map_err(map_crypto)?;

    let secret_bytes = cert.as_tsk().armored().to_vec().map_err(map_crypto)?;

    Ok((cert, secret_bytes))
}

pub fn cert_from_bytes(bytes: &[u8]) -> Result<Cert> {
    Cert::from_bytes(bytes).map_err(map_crypto)
}

pub fn import_secret_key(armored: &str) -> Result<Cert> {
    let cert = Cert::from_reader(armored.as_bytes()).map_err(map_crypto)?;
    if !cert.is_tsk() {
        return Err(CoreError::Crypto(
            "no secret key material in imported key".to_string(),
        ));
    }
    let policy = StandardPolicy::new();
    let usable = cert
        .keys()
        .unencrypted_secret()
        .with_policy(&policy, None)
        .for_transport_encryption()
        .for_storage_encryption()
        .next()
        .is_some();
    if !usable {
        let protected = cert
            .keys()
            .secret()
            .with_policy(&policy, None)
            .for_transport_encryption()
            .for_storage_encryption()
            .next()
            .is_some();
        if protected {
            return Err(CoreError::Crypto(
                "secret key is passphrase-protected; export it without a passphrase".to_string(),
            ));
        }
        return Err(CoreError::Crypto(
            "no decryption-capable secret key in imported key".to_string(),
        ));
    }
    Ok(cert)
}

pub fn encrypt(plaintext: &[u8], recipients: &[Cert]) -> Result<Vec<u8>> {
    let policy = StandardPolicy::new();

    let mut recipient_keys = Vec::new();
    for cert in recipients {
        let keys = cert
            .keys()
            .with_policy(&policy, None)
            .supported()
            .alive()
            .revoked(false)
            .for_transport_encryption();
        for key in keys {
            recipient_keys.push(key);
        }
    }

    if recipient_keys.is_empty() {
        return Err(CoreError::Crypto(
            "no transport-encryption keys among recipients".to_string(),
        ));
    }

    let mut ciphertext = Vec::new();
    let message = Message::new(&mut ciphertext);
    let message = Encryptor::for_recipients(message, recipient_keys)
        .build()
        .map_err(map_crypto)?;
    let mut message = LiteralWriter::new(message).build().map_err(map_crypto)?;
    message.write_all(plaintext)?;
    message.finalize().map_err(map_crypto)?;

    Ok(ciphertext)
}

pub fn decrypt(ciphertext: &[u8], key_cert: &Cert) -> Result<Vec<u8>> {
    let policy = StandardPolicy::new();
    let helper = DecryptHelper {
        secret: key_cert,
        policy: &policy,
    };

    let mut decryptor = DecryptorBuilder::from_bytes(ciphertext)
        .map_err(map_crypto)?
        .with_policy(&policy, None, helper)
        .map_err(map_crypto)?;

    let mut plaintext = Vec::new();
    std::io::copy(&mut decryptor, &mut plaintext)?;

    Ok(plaintext)
}

struct DecryptHelper<'a> {
    secret: &'a Cert,
    policy: &'a dyn Policy,
}

impl VerificationHelper for DecryptHelper<'_> {
    fn get_certs(&mut self, _ids: &[KeyHandle]) -> sequoia_openpgp::Result<Vec<Cert>> {
        Ok(Vec::new())
    }

    fn check(&mut self, _structure: MessageStructure) -> sequoia_openpgp::Result<()> {
        Ok(())
    }
}

impl DecryptionHelper for DecryptHelper<'_> {
    fn decrypt(
        &mut self,
        pkesks: &[sequoia_openpgp::packet::PKESK],
        _skesks: &[sequoia_openpgp::packet::SKESK],
        sym_algo: Option<SymmetricAlgorithm>,
        decrypt: &mut dyn FnMut(Option<SymmetricAlgorithm>, &SessionKey) -> bool,
    ) -> sequoia_openpgp::Result<Option<Cert>> {
        let candidates: Vec<_> = self
            .secret
            .keys()
            .unencrypted_secret()
            .with_policy(self.policy, None)
            .for_transport_encryption()
            .for_storage_encryption()
            .collect();

        for key in candidates {
            let mut pair = key.key().clone().into_keypair()?;
            for pkesk in pkesks {
                if pkesk
                    .decrypt(&mut pair, sym_algo)
                    .map(|(algo, session_key)| decrypt(algo, &session_key))
                    .unwrap_or(false)
                {
                    return Ok(None);
                }
            }
        }

        Err(sequoia_openpgp::Error::MissingSessionKey(
            "no matching decryption key in provided cert".to_string(),
        )
        .into())
    }
}
