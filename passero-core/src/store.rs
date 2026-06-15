use std::path::Path;
use crate::{crypto, Result, CoreError};
use sequoia_openpgp::cert::Cert;

pub fn init(store: &Path, gpg_ids: &[String]) -> Result<()> {
    std::fs::create_dir_all(store)?;
    std::fs::write(store.join(".gpg-id"), gpg_ids.join("\n") + "\n")?;
    Ok(())
}

pub fn list(store: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    walk(store, store, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') { continue; }
        if path.is_dir() { walk(root, &path, out)?; }
        else if path.extension().and_then(|s| s.to_str()) == Some("gpg") {
            let rel = path.strip_prefix(root).unwrap().with_extension("");
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

pub fn show(store: &Path, entry: &str, key: &Cert) -> Result<Vec<u8>> {
    let file = store.join(entry).with_extension("gpg");
    let ciphertext = std::fs::read(file)?;
    crypto::decrypt(&ciphertext, key)
}

pub fn insert(store: &Path, entry: &str, plaintext: &[u8], recipients: &[Cert]) -> Result<()> {
    let file = store.join(entry).with_extension("gpg");
    if let Some(parent) = file.parent() { std::fs::create_dir_all(parent)?; }
    let ciphertext = crypto::encrypt(plaintext, recipients)?;
    std::fs::write(file, ciphertext)?;
    Ok(())
}

pub fn recipients_for(store: &Path, _entry: &str) -> Result<Vec<String>> {
    let gpg_id = store.join(".gpg-id");
    let text = std::fs::read_to_string(gpg_id)
        .map_err(|e| CoreError::Store(format!("no .gpg-id: {e}")))?;
    Ok(text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}
