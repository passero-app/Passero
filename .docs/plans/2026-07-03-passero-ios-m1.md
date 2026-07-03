# Passero iOS M1: GitHub Sign-In + Key Import Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace pasted-PAT GitHub auth with GitHub App device-flow sign-in, and let users import an existing armored PGP secret key — both stored in the iOS keychain behind the fixed multi-secret keystore.

**Architecture:** Three layers, built bottom-up. (1) Vendor and fix the `tauri-plugin-keystore` plugin so it can hold multiple named secrets with per-secret biometric policy — today it hardcodes ONE keychain account, which blocks everything else. (2) A `github` module in `passero-core` implements OAuth device flow (pure HTTP, unit-testable off-device); Tauri commands in `src-tauri/src/ios/mod.rs` drive it and persist tokens in the keystore instead of plaintext config. (3) Key import is a thin validation command over Sequoia plus a UI path that reuses the existing generate-key storage flow. YubiKey NFC is deferred to M2 (outline at end).

**Tech Stack:** Tauri 2 (iOS), Rust (sequoia-openpgp 2.x `crypto-rust`, git2, ureq for HTTP), Swift (keychain plugin), Svelte 5 frontend. GitHub App with device flow (no client secret, no backend).

**Branch:** create `m1-github-key-import` off `main` after PR #15 (iOS build fixes) is merged — the iOS build does not work without it.

**Key facts discovered during research (do not rediscover):**
- `passero-core/src/sync.rs:18` authenticates as `Cred::userpass_plaintext("x-access-token", tok)` — GitHub App user-to-server tokens work in the same position as PATs. **sync.rs needs no changes.**
- The published `tauri-plugin-keystore 2.1.0-alpha.1` iOS Swift impl hardcodes keychain account `"com.impierce.identity-wallet.unime-dev"` for store/retrieve/remove; `StoreRequest` has no service/user fields at all; desktop `store` hardcodes `keyring::Entry::new("com.impierce.identity-wallet", "tester")`. Registry copy: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-plugin-keystore-2.1.0-alpha.1/`.
- The M0 PAT is persisted in **plaintext** app config (`src-tauri/src/config/model.rs` field `pat: Option<String>`, read via `config::get_pat`). M1 moves tokens to the keystore; config `pat` is migrated then cleared.
- Frontend keystore calls live only in `src/views/M0Setup.svelte` (lines ~63, ~78, ~98; constants at top).
- Repo rule: **no code comments** in implementation code.
- After any `tauri ios init`, run `src-tauri/scripts/patch-ios-project.sh` (see `.docs/plans/2026-06-13-passero-ios-m0.md`).
- On-device dev runs: `npm run tauri ios dev -- "Fred’s iPhone" --host` (device name is positional and BEFORE `--host`, which greedily eats a following value). Personal Hotspot on the phone must be OFF or the device disappears from CoreDevice.

---

## Task 1: Vendor the keystore plugin

**Files:**
- Create: `src-tauri/plugins/tauri-plugin-keystore/` (copied from cargo registry)
- Modify: `src-tauri/Cargo.toml:39`

**Step 1: Copy the plugin source**

```bash
mkdir -p src-tauri/plugins
cp -R ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-plugin-keystore-2.1.0-alpha.1 src-tauri/plugins/tauri-plugin-keystore
cd src-tauri/plugins/tauri-plugin-keystore
rm -rf .cargo-ok .cargo_vcs_info.json Cargo.toml.orig Cargo.lock pnpm-lock.yaml .releaserc.yaml android
```

Remove the `android` dir reference from `src-tauri/plugins/tauri-plugin-keystore/build.rs` if it breaks the build (keep iOS + desktop paths).

**Step 2: Point Cargo at the vendored copy**

In `src-tauri/Cargo.toml` replace:

```toml
tauri-plugin-keystore = "=2.1.0-alpha.1"
```

with:

```toml
tauri-plugin-keystore = { path = "plugins/tauri-plugin-keystore" }
```

**Step 3: Verify the desktop build still compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean check (warnings OK), no missing-crate errors.

**Step 4: Commit**

```bash
git add src-tauri/plugins src-tauri/Cargo.toml Cargo.lock
git commit -m "build: vendor tauri-plugin-keystore for multi-secret fix"
```

---

## Task 2: Multi-secret keystore — Rust API

**Files:**
- Modify: `src-tauri/plugins/tauri-plugin-keystore/src/models.rs`
- Modify: `src-tauri/plugins/tauri-plugin-keystore/src/desktop.rs`

**Step 1: Extend the request models**

Replace `StoreRequest` in `models.rs`:

```rust
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreRequest {
    pub service: String,
    pub user: String,
    pub value: String,
    #[serde(default = "default_biometric")]
    pub biometric: bool,
}

fn default_biometric() -> bool {
    true
}
```

`RetrieveRequest` / `RemoveRequest` already carry `service` + `user` — leave them.

**Step 2: Fix the desktop impl to honor service/user**

In `desktop.rs`, replace the `store` body:

```rust
pub fn store(&self, payload: StoreRequest) -> crate::Result<()> {
    let entry = keyring::Entry::new(&payload.service, &payload.user).unwrap();
    entry.set_password(&payload.value).unwrap();
    Ok(())
}
```

Also make `retrieve` return `Ok(RetrieveResponse { value: None })` instead of panicking when `entry.get_password()` errors (missing item is an expected state):

```rust
pub fn retrieve(&self, payload: RetrieveRequest) -> crate::Result<RetrieveResponse> {
    let entry = keyring::Entry::new(&payload.service, &payload.user).unwrap();
    Ok(RetrieveResponse { value: entry.get_password().ok() })
}
```

**Step 3: Verify compile**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

**Step 4: Commit**

```bash
git add src-tauri/plugins/tauri-plugin-keystore/src
git commit -m "feat(keystore): multi-secret store API with per-secret biometric flag"
```

---

## Task 3: Multi-secret keystore — Swift (iOS)

**Files:**
- Modify: `src-tauri/plugins/tauri-plugin-keystore/ios/Sources/KeystorePlugin.swift`

**Step 1: Rewrite the plugin to key items by service/user and honor `biometric`**

Replace the class body. Decodables:

```swift
class StoreRequest: Decodable {
  let service: String
  let user: String
  let value: String
  let biometric: Bool?
}

class ItemRequest: Decodable {
  let service: String
  let user: String
}
```

`store`: build the query with `kSecAttrService as String: args.service` and `kSecAttrAccount as String: args.user`. Only attach the `SecAccessControlCreateWithFlags(nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, .userPresence, &error)` access control when `args.biometric ?? true` is true; otherwise set `kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly` instead. Keep the delete-then-add pattern.

`retrieve` and `remove`: parse `ItemRequest` and include BOTH `kSecAttrService` and `kSecAttrAccount` in the query (the current code queries account only). Keep the `LAContext`/`kSecUseOperationPrompt` behavior for retrieve as-is so biometric items prompt FaceID.

**Step 2: Verify iOS compile**

```bash
npm run tauri ios build -- --debug --target aarch64-sim
```
Expected: `** BUILD SUCCEEDED **`. Swift errors surface in this build — fix before proceeding.

**Step 3: Commit**

```bash
git add src-tauri/plugins/tauri-plugin-keystore/ios
git commit -m "feat(keystore): iOS keychain items keyed by service/user, optional biometric"
```

---

## Task 4: Frontend keystore wrapper + M0Setup migration

**Files:**
- Create: `src/lib/keystore.ts`
- Modify: `src/views/M0Setup.svelte` (keystore import at line 3, calls at ~63/78/98)
- Modify: `package.json` (drop `@impierce/tauri-plugin-keystore`)

**Step 1: Write the wrapper**

```typescript
import { invoke } from "@tauri-apps/api/core";

export async function store(
  service: string,
  user: string,
  value: string,
  opts: { biometric?: boolean } = {},
): Promise<void> {
  await invoke("plugin:keystore|store", {
    payload: { service, user, value, biometric: opts.biometric ?? true },
  });
}

export async function retrieve(service: string, user: string): Promise<string | null> {
  const r = await invoke<{ value: string | null }>("plugin:keystore|retrieve", {
    payload: { service, user },
  });
  return r.value ?? null;
}

export async function remove(service: string, user: string): Promise<void> {
  await invoke("plugin:keystore|remove", { payload: { service, user } });
}
```

**Step 2: Switch M0Setup.svelte to the wrapper**

- Line 3: `import { store, retrieve, remove } from "../lib/keystore";`
- The `store(generated.armored)` call becomes `store(KEYSTORE_SERVICE, KEYSTORE_USER, generated.armored)`.
- `retrieve`/`remove` call sites already pass `(KEYSTORE_SERVICE, KEYSTORE_USER)` — unchanged.

**Step 3: Remove the npm dependency**

```bash
npm uninstall @impierce/tauri-plugin-keystore
npm run check
```
Expected: svelte-check passes with no new errors.

**Step 4: On-device smoke test** (needs phone; hotspot OFF)

```bash
npm run tauri ios dev -- "Fred’s iPhone" --host
```
On the phone: reset device state if offered, generate a key, confirm FaceID unlock round-trips. The old M0 secret under the hardcoded Impierce account is orphaned — that is expected; no migration (M0 device data is test data).

**Step 5: Commit**

```bash
git add src/lib/keystore.ts src/views/M0Setup.svelte package.json package-lock.json
git commit -m "feat: local keystore wrapper with service/user addressing"
```

---

## Task 5: Device-flow client in passero-core (TDD)

**Files:**
- Create: `passero-core/src/github.rs`
- Modify: `passero-core/src/lib.rs` (add `pub mod github;`)
- Modify: `passero-core/Cargo.toml` (add deps)
- Test: `passero-core/tests/github.rs`

**Step 1: Add dependencies**

In `passero-core/Cargo.toml` `[dependencies]`:

```toml
ureq = { version = "2", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`[dev-dependencies]`: add `mockito = "1"`.

**Step 2: Write the failing tests**

`passero-core/tests/github.rs`:

```rust
use passero_core::github::{request_device_code, poll_once, refresh, PollResult};

#[test]
fn device_code_request_parses() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/device/code")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"device_code":"dc123","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#)
        .create();
    let dc = request_device_code(&server.url(), "client123").unwrap();
    assert_eq!(dc.user_code, "ABCD-1234");
    assert_eq!(dc.device_code, "dc123");
    assert_eq!(dc.interval, 5);
}

#[test]
fn poll_pending_then_token() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"error":"authorization_pending"}"#)
        .create();
    assert!(matches!(
        poll_once(&server.url(), "client123", "dc123").unwrap(),
        PollResult::Pending
    ));

    let _m2 = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"access_token":"ghu_tok","expires_in":28800,"refresh_token":"ghr_ref","token_type":"bearer"}"#)
        .create();
    match poll_once(&server.url(), "client123", "dc123").unwrap() {
        PollResult::Token(t) => {
            assert_eq!(t.access_token, "ghu_tok");
            assert_eq!(t.refresh_token.as_deref(), Some("ghr_ref"));
            assert_eq!(t.expires_in, Some(28800));
        }
        other => panic!("expected token, got {other:?}"),
    }
}

#[test]
fn poll_denied_is_error() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"error":"access_denied"}"#)
        .create();
    assert!(poll_once(&server.url(), "client123", "dc123").is_err());
}

#[test]
fn refresh_parses() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"access_token":"ghu_new","expires_in":28800,"refresh_token":"ghr_new","token_type":"bearer"}"#)
        .create();
    let t = refresh(&server.url(), "client123", "ghr_old").unwrap();
    assert_eq!(t.access_token, "ghu_new");
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p passero-core --test github`
Expected: compile FAILURE — `github` module does not exist.

**Step 4: Implement `passero-core/src/github.rs`**

```rust
use serde::Deserialize;
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug)]
pub enum PollResult {
    Pending,
    SlowDown,
    Token(Token),
}

#[derive(Debug, Deserialize)]
struct TokenOrError {
    error: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

pub fn request_device_code(base: &str, client_id: &str) -> Result<DeviceCode> {
    let resp: DeviceCode = ureq::post(&format!("{base}/login/device/code"))
        .set("Accept", "application/json")
        .send_form(&[("client_id", client_id)])
        .map_err(|e| Error::Http(e.to_string()))?
        .into_json()
        .map_err(|e| Error::Http(e.to_string()))?;
    Ok(resp)
}

pub fn poll_once(base: &str, client_id: &str, device_code: &str) -> Result<PollResult> {
    let resp: TokenOrError = ureq::post(&format!("{base}/login/oauth/access_token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .map_err(|e| Error::Http(e.to_string()))?
        .into_json()
        .map_err(|e| Error::Http(e.to_string()))?;
    match resp.error.as_deref() {
        Some("authorization_pending") => Ok(PollResult::Pending),
        Some("slow_down") => Ok(PollResult::SlowDown),
        Some(e) => Err(Error::Http(format!("device flow: {e}"))),
        None => Ok(PollResult::Token(Token {
            access_token: resp.access_token.ok_or_else(|| Error::Http("missing access_token".into()))?,
            refresh_token: resp.refresh_token,
            expires_in: resp.expires_in,
        })),
    }
}

pub fn refresh(base: &str, client_id: &str, refresh_token: &str) -> Result<Token> {
    let resp: TokenOrError = ureq::post(&format!("{base}/login/oauth/access_token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .map_err(|e| Error::Http(e.to_string()))?
        .into_json()
        .map_err(|e| Error::Http(e.to_string()))?;
    if let Some(e) = resp.error {
        return Err(Error::Http(format!("refresh: {e}")));
    }
    Ok(Token {
        access_token: resp.access_token.ok_or_else(|| Error::Http("missing access_token".into()))?,
        refresh_token: resp.refresh_token,
        expires_in: resp.expires_in,
    })
}
```

Add an `Http(String)` variant to `passero-core`'s error enum (check `passero-core/src/lib.rs` for how `Error`/`Result` are defined; mirror an existing variant).

The `base` parameter exists so tests can point at mockito; production callers pass `"https://github.com"`.

**Step 5: Run tests to verify they pass**

Run: `cargo test -p passero-core --test github`
Expected: 4 passed.

**Step 6: Commit**

```bash
git add passero-core
git commit -m "feat(core): GitHub device-flow client"
```

---

## Task 6: Register the GitHub App (manual, one-time)

No files. Human does this in a browser; takes ~3 minutes. GitHub Apps are free on org Free plans.

1. Go to `https://github.com/organizations/passero-app/settings/apps` → **New GitHub App**.
2. Fields:
   - **App name:** `Passero`
   - **Homepage URL:** `https://github.com/passero-app/Passero`
   - **Callback URL:** leave empty. UNCHECK "Request user authorization (OAuth) during installation".
   - **CHECK "Enable Device Flow"** ← the whole point.
   - **Webhook:** UNCHECK "Active".
   - **Repository permissions:** Contents → **Read and write**. (Metadata Read-only is added automatically.)
   - **Where can this app be installed:** Any account (so personal-account stores work too).
3. Create the app. Copy the **Client ID** (looks like `Iv23li…`) — it is public, not a secret.
4. Install the app: app settings → Install App → choose the account owning the password-store repo → **Only select repositories** → pick the store repo.
5. Record the Client ID for Task 7.

---

## Task 7: Token storage + commands (replaces plaintext PAT)

**Files:**
- Modify: `src-tauri/src/ios/mod.rs`
- Modify: `src-tauri/src/lib.rs` (register new commands in `invoke_handler`, ~line 100)

**Design:** one keystore entry, service `app.passero`, user `github-auth`, `biometric: false` (sync must not FaceID-prompt), value = JSON:

```json
{"access_token":"…","refresh_token":"…","expires_at":1751600000}
```

`expires_at` = unix seconds computed at store time (`SystemTime::now() + expires_in - 60`); `null` for non-expiring manual PATs.

**Step 1: Add the client-id const and auth structs to `ios/mod.rs`**

```rust
const GITHUB_CLIENT_ID: &str = "FILL_ME_FROM_TASK_6";
const GITHUB_BASE: &str = "https://github.com";

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct GithubAuth {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
}
```

Add to `IosState`: `pub device_login: Mutex<Option<(String, u64)>>` (device_code, interval).

**Step 2: Keystore access from Rust**

The plugin exposes `app.keystore()` via its `KeystoreExt` trait (see `plugins/tauri-plugin-keystore/src/lib.rs`). Add helpers in `ios/mod.rs`:

```rust
fn save_github_auth(app: &tauri::AppHandle, auth: &GithubAuth) -> Result<()>
fn read_github_auth(app: &tauri::AppHandle) -> Result<Option<GithubAuth>>
```

implemented with `app.keystore().store(StoreRequest { service: "app.passero".into(), user: "github-auth".into(), value: serde_json::to_string(auth)?, biometric: false })` and the matching retrieve.

**Step 3: Commands**

```rust
#[tauri::command]
pub async fn github_login_start(state: State<'_, IosState>) -> Result<serde_json::Value> {
    let dc = passero_core::github::request_device_code(GITHUB_BASE, GITHUB_CLIENT_ID)
        .map_err(|e| PasseroError::GitError(e.to_string()))?;
    *state.device_login.lock().unwrap() = Some((dc.device_code.clone(), dc.interval));
    Ok(serde_json::json!({"userCode": dc.user_code, "verificationUri": dc.verification_uri, "interval": dc.interval}))
}

#[tauri::command]
pub async fn github_login_poll(app: tauri::AppHandle, state: State<'_, IosState>) -> Result<String> {
    let (device_code, _) = state.device_login.lock().unwrap().clone()
        .ok_or_else(|| PasseroError::GitError("no login in progress".into()))?;
    match passero_core::github::poll_once(GITHUB_BASE, GITHUB_CLIENT_ID, &device_code)
        .map_err(|e| PasseroError::GitError(e.to_string()))? {
        passero_core::github::PollResult::Pending => Ok("pending".into()),
        passero_core::github::PollResult::SlowDown => Ok("slow_down".into()),
        passero_core::github::PollResult::Token(t) => {
            let expires_at = t.expires_in.map(|s| now_unix() + s.saturating_sub(60));
            save_github_auth(&app, &GithubAuth { access_token: t.access_token, refresh_token: t.refresh_token, expires_at })?;
            *state.device_login.lock().unwrap() = None;
            Ok("authorized".into())
        }
    }
}

#[tauri::command]
pub async fn github_logout(app: tauri::AppHandle, state: State<'_, IosState>) -> Result<()>
```

(`github_logout` removes the keystore entry and clears `state.token`.)

**Step 4: `get_valid_token` — the one accessor every git op uses**

```rust
fn get_valid_token(app: &tauri::AppHandle) -> Result<Option<String>> {
    let Some(mut auth) = read_github_auth(app)? else {
        return legacy_pat_migration(app);
    };
    if let (Some(exp), Some(refresh_token)) = (auth.expires_at, auth.refresh_token.clone()) {
        if now_unix() >= exp {
            let t = passero_core::github::refresh(GITHUB_BASE, GITHUB_CLIENT_ID, &refresh_token)
                .map_err(|e| PasseroError::GitError(e.to_string()))?;
            auth = GithubAuth {
                access_token: t.access_token,
                refresh_token: t.refresh_token,
                expires_at: t.expires_in.map(|s| now_unix() + s.saturating_sub(60)),
            };
            save_github_auth(app, &auth)?;
        }
    }
    Ok(Some(auth.access_token))
}
```

`legacy_pat_migration`: if `config::get_pat(&app)?` is `Some(pat)`, save it as `GithubAuth { access_token: pat, refresh_token: None, expires_at: None }`, call `config::set_sync_settings(&app, None, Some(String::new()))` (or however the config layer clears the field — check `src-tauri/src/config/`), and return it.

**Step 5: Rewire the git-op commands**

- `load_key` (line ~352): replace the `config::get_pat` block with `get_valid_token`.
- `clone_store` (line ~366): empty-token fallback becomes `get_valid_token(&app)?.unwrap_or_default()`; stop passing `&token` into `config::register_cloned_vault` if that persists it in plaintext (check `src-tauri/src/config/` and strip PAT persistence there).
- `set_sync_settings` (line ~386): a manually pasted PAT now goes to `save_github_auth` (expires_at None) instead of config. Keep the command as the escape hatch.
- `get_sync_settings` (line ~379): `has_pat` becomes `read_github_auth(&app)?.is_some()`.
- Register `github_login_start`, `github_login_poll`, `github_logout` in `src-tauri/src/lib.rs` `invoke_handler`.

**Step 6: Compile + desktop tests**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && cargo test --workspace`
Expected: clean check; existing tests pass.

**Step 7: Commit**

```bash
git add src-tauri/src passero-core
git commit -m "feat(ios): GitHub device-flow login, tokens in keystore, plaintext PAT migration"
```

---

## Task 8: Sign-in UI

**Files:**
- Modify: `src/views/M0Setup.svelte` (clone/PAT section)

**Step 1: Replace the PAT text field with device-flow UI**

State machine: `idle → code_shown → authorized | failed`.

```svelte
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";

  let ghState: "idle" | "code" | "authorized" | "failed" = $state("idle");
  let userCode = $state("");
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function githubSignIn() {
    const r = await invoke<{ userCode: string; verificationUri: string; interval: number }>(
      "github_login_start",
    );
    userCode = r.userCode;
    ghState = "code";
    await open(r.verificationUri);
    pollTimer = setInterval(async () => {
      const s = await invoke<string>("github_login_poll").catch(() => "failed");
      if (s === "authorized" || s === "failed") {
        clearInterval(pollTimer!);
        ghState = s as typeof ghState;
      }
    }, (r.interval + 1) * 1000);
  }
</script>
```

UI: a "Sign in with GitHub" button; while `code`, show `userCode` large + "Enter this code on github.com/login/device" (Safari opens automatically); on `authorized`, show ✓ and enable the clone step with no token field. Keep an "Advanced: paste a token" disclosure that calls the existing `set_sync_settings`.

**Step 2: Type-check**

Run: `npm run check`
Expected: no new errors.

**Step 3: On-device verification** (requires Task 6's Client ID filled in)

`npm run tauri ios dev -- "Fred’s iPhone" --host` → tap Sign in with GitHub → Safari opens → enter code → app flips to authorized → clone the store repo without pasting anything → pull/push round-trip works.

**Step 4: Commit**

```bash
git add src/views/M0Setup.svelte
git commit -m "feat(ui): GitHub device-flow sign-in replaces PAT paste"
```

---

## Task 9: Armored secret-key import in passero-core (TDD)

**Files:**
- Modify: `passero-core/src/crypto.rs`
- Test: `passero-core/tests/import.rs`

**Step 1: Write the failing tests**

```rust
use passero_core::crypto::{generate_key, import_secret_key};

#[test]
fn import_roundtrips_generated_key() {
    let (cert, armored) = generate_key("test <t@example.com>").unwrap();
    let imported = import_secret_key(std::str::from_utf8(&armored).unwrap()).unwrap();
    assert_eq!(imported.fingerprint(), cert.fingerprint());
}

#[test]
fn import_rejects_public_only() {
    let (cert, _) = generate_key("test <t@example.com>").unwrap();
    let mut public = Vec::new();
    cert.armored().serialize(&mut public).unwrap();
    assert!(import_secret_key(std::str::from_utf8(&public).unwrap()).is_err());
}

#[test]
fn import_rejects_garbage() {
    assert!(import_secret_key("not a key").is_err());
}
```

(Adjust the `generate_key` signature/`serialize` import to match `crypto.rs:22` and existing test style in `passero-core/tests/` — read one existing test file first.)

**Step 2: Run to verify failure**

Run: `cargo test -p passero-core --test import`
Expected: compile FAILURE — `import_secret_key` not found.

**Step 3: Implement**

In `crypto.rs`:

```rust
pub fn import_secret_key(armored: &str) -> Result<Cert> {
    let cert = Cert::from_reader(armored.as_bytes())?;
    if !cert.is_tsk() {
        return Err(Error::Crypto("no secret key material in imported key".into()));
    }
    Ok(cert)
}
```

(Match the crate's real error variant names — check how `cert_from_bytes` at `crypto.rs:34` maps errors.)

**Step 4: Run to verify pass**

Run: `cargo test -p passero-core --test import`
Expected: 3 passed.

**Step 5: Commit**

```bash
git add passero-core
git commit -m "feat(core): validate and import armored secret keys"
```

---

## Task 10: Import command + UI

**Files:**
- Modify: `src-tauri/src/ios/mod.rs`
- Modify: `src-tauri/src/lib.rs` (register command)
- Modify: `src/views/M0Setup.svelte`

**Step 1: Command**

The M0 generate flow already does: rust returns armored → frontend stores in keystore → frontend calls `load_key`. Import reuses that shape — the command only validates and normalizes:

```rust
#[tauri::command]
pub async fn validate_secret_key(armored: String) -> Result<serde_json::Value> {
    let cert = passero_core::crypto::import_secret_key(&armored)
        .map_err(|e| PasseroError::GpgError(e.to_string()))?;
    Ok(serde_json::json!({"fingerprint": cert.fingerprint().to_hex()}))
}
```

Register in `lib.rs`.

**Step 2: UI**

In M0Setup.svelte next to "Generate key": an "Import existing key" disclosure with a `<textarea>` for the armored block and an Import button:

```typescript
async function importKey() {
  const r = await invoke<{ fingerprint: string }>("validate_secret_key", { armored: pasted });
  await store(KEYSTORE_SERVICE, KEYSTORE_USER, pasted);
  await invoke("load_key", { armored: pasted });
  fingerprint = r.fingerprint;
}
```

Show the fingerprint for confirmation before/after storing (match the generate flow's UX). Wipe the textarea variable after storing.

**Step 3: Type-check + tests**

Run: `npm run check && cargo test --workspace`
Expected: clean.

**Step 4: On-device verification**

Export a test key on the desktop (`gpg --armor --export-secret-keys <test key>` or use Passero desktop), paste into the phone UI, confirm FaceID-gated unlock decrypts a store entry encrypted to that key.

**Step 5: Commit**

```bash
git add src-tauri/src src/views/M0Setup.svelte
git commit -m "feat: import armored PGP secret key behind FaceID"
```

---

## Task 11 (DEFERRED — M2 outline, do not implement in M1): YubiKey over NFC

PGP private keys on a YubiKey are non-exportable by design — "import from YubiKey" is impossible. The real feature is **on-card crypto**: the app sends decrypt/sign operations to the YubiKey over NFC and the key never exists on the phone.

Shape of the work:
- New Swift Tauri plugin wrapping Yubico's YubiKit (`NFCISO7816` smartcard session, OpenPGP applet AID `D27600012401`).
- Entitlements: NFC Tag Reading capability (already registered on the App ID) + `com.apple.developer.nfc.readersession.iso7816.select-identifiers` containing the OpenPGP AID in Info.plist (goes in `src-tauri/Info.ios.plist`).
- `passero-core/src/crypto.rs` grows a `Decryptor` abstraction so `decrypt` can delegate the private-key operation to either the in-memory Cert (today's path) or a card session (sequoia's `crypto::Decryptor` trait is the hook).
- UX: "Unlock with YubiKey" → NFC sheet → tap card per decrypt (or per session with cached session key).
- Estimate: its own milestone; plan separately when M1 lands.

---

## Execution notes

- Tasks 1–4 are the foundation; nothing else works without them. Task 5 and Task 9 are independent of each other and of Task 6 (parallelizable). Task 7 needs 5+6; Task 8 needs 7; Task 10 needs 9 and 4.
- Every on-device step: Personal Hotspot OFF, phone unlocked, `npm run tauri ios dev -- "Fred’s iPhone" --host`.
- If `src-tauri/gen/apple` is ever regenerated: `npm run tauri ios init && src-tauri/scripts/patch-ios-project.sh`.
