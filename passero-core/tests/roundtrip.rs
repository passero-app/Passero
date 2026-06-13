#[test]
fn core_crate_builds_and_links() {
    assert_eq!(passero_core::version_tag(), "passero-core");
}

use passero_core::crypto;

#[test]
fn encrypt_then_decrypt_roundtrips() {
    let (cert, _secret_bytes) = crypto::generate_key("test@passero.local").unwrap();
    let plaintext = b"correct horse battery staple";

    let ciphertext = crypto::encrypt(plaintext, &[cert.clone()]).unwrap();
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = crypto::decrypt(&ciphertext, &cert).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_with_wrong_key_fails() {
    let (recipient, _) = crypto::generate_key("a@passero.local").unwrap();
    let (stranger, _) = crypto::generate_key("b@passero.local").unwrap();
    let ciphertext = crypto::encrypt(b"secret", &[recipient]).unwrap();
    assert!(crypto::decrypt(&ciphertext, &stranger).is_err());
}

#[test]
fn key_serializes_and_reloads() {
    let (cert, secret_bytes) = crypto::generate_key("c@passero.local").unwrap();
    let reloaded = crypto::cert_from_bytes(&secret_bytes).unwrap();
    assert_eq!(cert.fingerprint(), reloaded.fingerprint());
}
