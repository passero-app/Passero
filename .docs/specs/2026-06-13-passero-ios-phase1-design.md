# Passero iOS — Phase 1 Design

**Date:** 2026-06-13
**Status:** Approved (design); pending implementation plan
**Scope:** Phase 1 — read/use iOS client with a phone-resident software key
**Author:** Fred Smith (with Claude)

## 1. Context & Motivation

Passero is a desktop GUI for [`pass`](https://www.passwordstore.org/), the standard Unix
password store: each secret is a file encrypted to one or more GPG recipient public keys,
the store is a git repository, and decryption keys live on the user's hardware (YubiKey) or
keyring — never in a cloud vault.

The goal is to make `pass` the single source of truth for the author's **personal** and
**family** password management, replacing the current situation where passwords are
duplicated into Apple Passwords (for family iPhones) and Bitwarden (for family
Windows/Linux/iOS) because `pass` has no good iOS story.

Two blockers keep that duplication alive:

1. **iOS access** — there is no first-class way to read/use the `pass` store on an iPhone.
   The existing third-party "Pass for iOS" app supports YubiKeys only for key formats the
   author's key does not use (his card key is ed25519/cv25519).
2. **Sharing** — family members live on iPhones, so a shared `pass` store is useless to
   them until (1) is solved. `pass` already supports multi-recipient sharing and Passero
   already manages recipients; the access gap is what blocks family adoption.

**iOS is the keystone.** Solving iOS access is expected to collapse most of the duplication
on its own, and turns family sharing from an access problem into a recipients-management UX
problem.

### Trust model (design principle)

The user does not trust any cloud provider — including Bitwarden — as the store of record.
`pass` earns that trust because **the sync layer only ever sees ciphertext**; decryption
keys never leave the user's devices. GitHub here is a dumb, blind, interchangeable sync
pipe, not a vault. The design principle that follows: **the sync layer is untrusted by
design** — GitHub, a self-hosted git remote, or a USB stick are equivalent and swappable.

## 2. Goals & Non-Goals

### Phase 1 goals
- A native iOS app that can pull a `pass` store from a git remote, decrypt entries, display
  TOTP, edit/insert entries, and push changes back.
- Keep the author's existing **ed25519/cv25519** key format — no re-keying required.
- Reuse the existing Passero **Svelte UI** unchanged behind the same command surface.
- A pure-Rust core that is unit-testable off-device, to avoid the flaky native-app failure
  surface.
- An in-app key-generation + vault-re-encryption flow that doubles as the family-onboarding
  mechanism.

### Non-goals (Phase 1) — deferred
- **YubiKey-NFC unlock on iOS** — high-risk, unproven; its own Phase 2 spike.
- **Full GPG key-management surface** on iOS (`publish`, `set_trust`, keyserver search,
  key deletion/export) — remains a desktop admin activity.
- **Migrating the desktop app onto the new core** — desktop keeps its current subprocess
  backend in Phase 1; unification is a later, deliberate step.

## 3. Key Decisions (approved)

| Decision | Choice | Rationale |
|---|---|---|
| PGP engine | **Sequoia** with the **pure-Rust crypto backend** (`crypto-rust`), not Nettle/C | Pure-Rust cross-compiles cleanly to `aarch64-apple-ios`; the C dependency is the classic iOS build trap. rPGP is the fallback if Sequoia-on-iOS resists. |
| Key storage | iOS **Keychain** with Secure-Enclave-gated access control (biometric/passcode) | Key never plaintext at rest; unlock gated by device auth. The Enclave wraps the key; it cannot *be* an OpenPGP key. |
| Git transport | **GitHub private repo over HTTPS + fine-grained PAT** (PAT in Keychain) | Simpler than SSH on iOS via `git2`. SSH can come later. |
| Core packaging | New pure-Rust **`passero-core`** crate; desktop untouched in Phase 1 | Isolates risk; desktop keeps working; core is reusable by both targets later. |
| Phase 1 key | **Phone-resident software key** (ed25519, generated in-app) | Accepted tradeoff for Phase 1; YubiKey unlock returns in Phase 2. |

## 4. Architecture

### 4.1 The command-surface seam

The Passero Svelte UI communicates with the Rust backend exclusively through ~40 Tauri
`invoke()` commands (`show_password`, `list_passwords`, `git_pull`, `list_recipients`,
`get_totp`, …). Only three desktop backend modules actually shell out to binaries
(`pass/store.rs` → `pass`, `gpg/commands.rs` → `gpg`, `git/commands.rs` → `git`); the
`totp/` module is already native Rust.

iOS cannot spawn those binaries. The plan reimplements the **same logical commands**
natively so the **UI cannot tell the difference**.

```
        ┌─────────────────────────────┐
        │  Svelte UI (shared, unchanged)
        └──────────────┬──────────────┘
                       │ invoke(...) — same command names
        ┌──────────────┴──────────────┐
        │   Tauri command handlers     │
        ├──────────────────────────────┤
        │        passero-core (Rust)    │
        │  ┌─────────┬────────┬───────┐ │
        │  │ store   │ crypto │ sync  │ │
        │  │ (pass   │(Sequoia│(git2) │ │
        │  │ layout) │ rust)  │       │ │
        │  └─────────┴────────┴───────┘ │
        │   totp (reuse totp-rs)        │
        │   keys (Keychain bridge)      │
        └──────────────────────────────┘
```

### 4.2 `passero-core` modules

- **`store`** — pass layout: walk the tree of `<entry>.gpg` files; parse `.gpg-id` files
  (recipient fingerprints) per subtree; map entry paths ↔ filesystem paths.
- **`crypto`** — Sequoia (pure-Rust backend): decrypt a `.gpg` file with the in-app key;
  encrypt plaintext to the recipient set for a subtree; generate an ed25519 cert; export a
  public cert.
- **`sync`** — `git2`/libgit2: clone, fetch+merge (pull), commit, push, log; HTTPS+PAT
  credentials callback.
- **`totp`** — reuse existing `totp-rs` logic.
- **`keys`** — bridge to iOS Keychain for the secret key and the GitHub PAT (Swift/FFI shim
  on iOS; on desktop a no-op or file-based equivalent if/when the core is shared).

## 5. Command Surface — Phase 1 subset

Implement natively (same names the UI already calls):

`list_passwords`, `show_password`, `insert_password`, `edit_password`, `delete_password`,
`copy_password`, `generate_password`, `get_totp`, `get_totp_info`, `git_pull`, `git_push`,
`git_clone`, `git_log`, `list_recipients`, `add_recipient`, `remove_recipient`,
`init_password_store`, plus a Phase-1 key command: `generate_in_app_key` (+ `export_public_cert`).

Out of scope for iOS Phase 1 (desktop-only): `publish_gpg_key`, `set_gpg_key_trust`,
`search_gpg_keyserver`, `import_gpg_key_from_keyserver`, `delete_gpg_key`, `export_gpg_key`.

## 6. Data Flow

**Read an entry:** UI `show_password(path)` → core resolves path → `git2` ensures local
clone present → read `<path>.gpg` bytes → `crypto.decrypt` with Keychain key (Enclave auth
prompt if locked) → return plaintext → UI renders. TOTP entries additionally run through
`totp-rs`.

**Edit/insert:** UI sends plaintext → core reads subtree `.gpg-id` recipients →
`crypto.encrypt` to all recipients → write `.gpg` → `git2` commit → (optional) push.

**Sync:** `git_pull` = fetch + fast-forward/merge; `git_push` = push committed changes.
Conflicts surface as errors for the user to resolve (Phase 1: last-writer guidance, no
auto-merge of ciphertext).

## 7. Milestone 0 — Validation Spike (riskiest assumption first)

Minimal/throwaway UI. Proves the native core end-to-end on a **real iPhone**, against a
**throwaway test vault** (separate git repo), with **zero contact** with the real store or
the real YubiKey key:

1. Generate a fresh ed25519 key in-app; store secret in Keychain.
2. Create/clone a throwaway test vault repo; `init` it (write `.gpg-id`) to the test key.
3. Encrypt a sample entry to the test key; commit; push.
4. On the phone: pull → **decrypt the entry** → edit → encrypt → commit → push.
5. Confirm round-trip from a second checkout (e.g., desktop) reads the edit.

**Exit criterion:** the round-trip succeeds on a physical iPhone. If it does, the hard
unknowns (Sequoia-on-iOS, git2-on-iOS, Keychain key use) are eliminated and the remainder is
UI assembly. This spike is also the literal **family-onboarding flow**: a new member
generates a key on their phone, their public cert is added as a recipient, the subtree is
re-encrypted, and they can decrypt.

## 8. Error Handling

- Crypto errors (wrong key / not a recipient / corrupt ciphertext) → typed errors surfaced
  to the UI with actionable messages; never panic across the FFI boundary.
- Keychain/Enclave auth failure or cancellation → distinct "locked/cancelled" state, not a
  generic failure.
- Git errors (auth, network, non-fast-forward) → typed and distinguishable so the UI can
  prompt re-auth vs. retry vs. resolve-conflict.
- All `passero-core` public functions return `Result` with a `thiserror` error enum
  (mirrors the existing desktop `error.rs` pattern).

## 9. Testing Strategy

The point of this architecture is testability without a device:

- **Unit (off-device, host target):** encrypt→decrypt round-trip with a generated key;
  `.gpg-id` parsing (single/multi-recipient, nested subtrees); store-tree walking and
  path mapping; TOTP derivation; git ops against a **local bare repo** (clone/commit/push/
  pull/log).
- **Negative tests:** decrypt with a non-recipient key; corrupt ciphertext; conflicting
  push.
- **On-device (manual, Milestone 0):** the full round-trip on a physical iPhone.
- Desktop app behavior is unaffected and its existing tests remain the regression guard for
  the subprocess backend.

## 10. Risks & Open Questions

- **Sequoia pure-Rust backend on iOS** — primary risk; mitigated by Milestone 0 and the
  rPGP fallback. *Open:* confirm feature flags and that `crypto-rust` covers ed25519/cv25519
  decryption fully.
- **`git2`/libgit2 iOS build** — well-trodden but needs cross-compile validation in
  Milestone 0.
- **Keychain ↔ Rust FFI** — thin Swift shim; the only Swift in Phase 1. Keep it minimal.
- **Conflict handling** — Phase 1 intentionally minimal (surface errors). Revisit if family
  concurrent edits prove common.
- **GitHub PAT lifecycle** — fine-grained PAT scope/expiry and re-auth UX; document setup.
- **Phase 2 dependency** — nothing in Phase 1 should preclude adding YubiKey-NFC unlock as
  an alternate key source behind the `keys` module.

## 11. Milestones (high-level; detailed plan to follow)

- **M0 — Validation spike** (Section 7). Gate for everything else.
- **M1 — `passero-core` read path:** clone, list, show, TOTP, against a real vault;
  off-device unit tests green.
- **M2 — write path:** insert/edit/generate/delete, encrypt-to-recipients, commit/push.
- **M3 — iOS UI integration:** wire the consumption-subset commands to the existing Svelte
  UI in the Tauri iOS target; vault setup + key generation/onboarding screens.
- **M4 — family onboarding polish:** recipient management UX for shared subtrees; multi-key
  test with a second generated key.

## 12. Phase 2 (out of scope here, recorded for continuity)

YubiKey-NFC unlock spike (CoreNFC/YubiKit ↔ OpenPGP applet APDUs bridged to the `keys`
module); full key-management surface on iOS if warranted; desktop migration onto
`passero-core` (removing the system `pass`/`gpg` install requirement).
