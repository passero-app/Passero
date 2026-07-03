use passero_core::crypto::{generate_key, import_secret_key};
use sequoia_openpgp::serialize::SerializeInto;

#[test]
fn import_roundtrips_generated_key() {
    let (cert, armored) = generate_key("test <t@example.com>").unwrap();
    let imported = import_secret_key(std::str::from_utf8(&armored).unwrap()).unwrap();
    assert_eq!(imported.fingerprint(), cert.fingerprint());
}

#[test]
fn import_rejects_public_only() {
    let (cert, _) = generate_key("test <t@example.com>").unwrap();
    let public = cert.armored().to_vec().unwrap();
    assert!(import_secret_key(std::str::from_utf8(&public).unwrap()).is_err());
}

#[test]
fn import_rejects_garbage() {
    assert!(import_secret_key("not a key").is_err());
}
