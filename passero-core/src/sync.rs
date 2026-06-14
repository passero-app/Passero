use git2::{Repository, RemoteCallbacks, FetchOptions, PushOptions, Cred, CredentialType, Signature};
use std::cell::Cell;
use std::path::Path;
use crate::Result;

fn callbacks(token: Option<&str>) -> RemoteCallbacks<'_> {
    let token = token.map(|t| t.to_string());
    let attempts = Cell::new(0usize);
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_url, _username, allowed| {
        let attempt = attempts.get();
        attempts.set(attempt + 1);
        if attempt > 0 {
            return Err(git2::Error::from_str("authentication failed"));
        }
        match &token {
            Some(tok) if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) => {
                Cred::userpass_plaintext("x-access-token", tok)
            }
            _ => Err(git2::Error::from_str("no credentials available")),
        }
    });
    cb
}

fn current_branch(repo: &Repository) -> String {
    repo.head().ok().and_then(|h| h.shorthand().map(String::from)).unwrap_or_else(|| "main".to_string())
}

pub fn clone(url: &str, into: &Path, token: Option<&str>) -> Result<Repository> {
    if Repository::open(into).is_ok() {
        return Ok(Repository::open(into)?);
    }
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks(token));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);
    let repo = builder.clone(url, into)?;
    if repo.head().is_err() {
        repo.set_head("refs/heads/main")?;
    }
    Ok(repo)
}

pub fn pull(repo: &Repository, token: Option<&str>) -> Result<()> {
    let b = current_branch(repo);
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks(token));
    repo.find_remote("origin")?.fetch(&[b.as_str()], Some(&mut fo), None)?;
    let fetch_head = repo.refname_to_id("FETCH_HEAD")?;
    let commit = repo.find_annotated_commit(fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&commit])?;
    if analysis.is_fast_forward() {
        let mut r = repo.find_reference(&format!("refs/heads/{b}"))?;
        r.set_target(fetch_head, "ff")?;
        repo.set_head(&format!("refs/heads/{b}"))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    }
    Ok(())
}

pub fn commit_all(repo: &Repository, msg: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let sig = Signature::now("Passero iOS", "passero@local")?;
    let parents = match repo.head().ok().and_then(|h| h.target()) {
        Some(oid) => vec![repo.find_commit(oid)?],
        None => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)?;
    Ok(())
}

pub fn push(repo: &Repository, token: Option<&str>) -> Result<()> {
    let b = current_branch(repo);
    let refspec = format!("refs/heads/{b}:refs/heads/{b}");
    let mut po = PushOptions::new();
    po.remote_callbacks(callbacks(token));
    repo.find_remote("origin")?
        .push(&[refspec.as_str()], Some(&mut po))?;
    Ok(())
}
