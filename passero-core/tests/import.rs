use passero_core::crypto::{generate_key, import_secret_key};
use sequoia_openpgp::cert::CertBuilder;
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

#[test]
fn import_rejects_passphrase_protected_key() {
    let (cert, _) = CertBuilder::new()
        .add_userid("test <t@example.com>")
        .add_transport_encryption_subkey()
        .set_password(Some("hunter2".into()))
        .generate()
        .unwrap();
    let armored = cert.as_tsk().armored().to_vec().unwrap();
    let err = import_secret_key(std::str::from_utf8(&armored).unwrap()).unwrap_err();
    assert!(err.to_string().contains("passphrase-protected"));
}

#[test]
fn import_rejects_signing_only_key() {
    let (cert, _) = CertBuilder::new()
        .add_userid("test <t@example.com>")
        .add_signing_subkey()
        .generate()
        .unwrap();
    let armored = cert.as_tsk().armored().to_vec().unwrap();
    let err = import_secret_key(std::str::from_utf8(&armored).unwrap()).unwrap_err();
    assert!(err.to_string().contains("no decryption-capable"));
}
