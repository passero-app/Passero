# M1 execution state — resume notes (2026-07-03, written at ~91% session usage)

Branch: `m1-github-key-import` (all listed commits pushed unless noted).
Plan: `.docs/plans/2026-07-03-passero-ios-m1.md`. Process: subagent per task, two-stage review, fixes re-reviewed.

## Done (implemented + reviewed + pushed)
- Task 1 vendor keystore plugin — 241f178
- Task 2 keystore Rust API — a73d279
- Task 3 keystore Swift (multi-secret, atomic store, null-on-missing) — 88e8894, 4e336e9, 018da51
- Task 4 frontend keystore wrapper — 4a1196c
- Task 5 device-flow client + timeout/test hardening — bcfc16a, 5b846a8
- Task 6 GitHub App registered: owner passero-app, App ID 4209539, **Client ID Iv23litEb2DKDwJNLkVX**, installed on fredsmith/password-store (Contents R/W)
- Task 7 auth commands + token keystore + PAT migration + per-op refresh — ecaf64a, 2580f3b
- Icons: sparrow icon recovered/regenerated, master committed as app-icon.png — 7989f41
- PR #15 merged to main (merge commit) — M1 branch diffs clean against main.

## In flight at time of writing (resume by checking their commits)
- Task 8 sign-in UI (agent in MAIN checkout, expected commit "feat(ui): GitHub device-flow sign-in replaces PAT paste" touching src/views/M0Setup.svelte + possibly src-tauri/capabilities/*.json). If commit exists but no review happened: review it (spec = plan Task 8 + backend contract below), then quality pass.
- Task 9 key import (agent in ISOLATED WORKTREE — find via `git worktree list`; expected commit "feat(core): validate and import armored secret keys" on the worktree branch touching passero-core/{src/crypto.rs,tests/import.rs}). Integrate by cherry-pick onto m1-github-key-import after review, then remove worktree.

## Remaining
- Reviews of 8+9 (deliberately deferred to post-2:40pm window)
- Task 10: validate_secret_key command + import UI (depends on 8 AND 9 both landed — touches ios/mod.rs, lib.rs, M0Setup.svelte; do NOT run parallel with anything touching M0Setup)
- Final: sim build green, push all, final whole-branch review, then Task 11 on-device smoke test (user + phone, Hotspot OFF): FaceID keystore roundtrip, GitHub sign-in → clone/pull/push tokenless, key import + decrypt, live refresh() sanity (no client_secret — contract risk), sparrow icon visible.

## OPEN BUGS from on-device testing (next session, in priority order)
1. **v6 key interop (CRITICAL for real use):** sequoia 2.x CertBuilder defaults to RFC 9580 (v6) keys; desktop GnuPG 2.4 fails with "packet(1) with unknown version 6" on anything the phone-generated key touches. Fix: `generate_key` in passero-core/src/crypto.rs must set the RFC 4880 profile (CertBuilder::set_profile? check sequoia 2.3 API name) so device keys are v4. Fred must regenerate the device key afterwards. Verify with a phone-encrypt → desktop `pass` decrypt roundtrip.
2. CORRECTION (Fred clarified: only ONE vault cloned; earlier second-vault theory wrong). The full failure model, confirmed by testing: .gpg-id correctly lists both fprs (phone 52A45938…, desktop 1D4BFCE3…), but (a) desktop gpg 2.4 cannot parse the phone's v6 pubkey, so desktop-side `pass insert` encrypts ONLY to the desktop key → phone cannot read desktop entries; (b) phone-encrypted entries involve v6 packets → desktop errors "unknown version 6". BOTH directions are the v6 root cause in bug 1. Fix chain: v4 generate_key → regenerate device key → export pubkey to desktop → re-run `pass init <both>` → re-encrypt. Verify during the fix that the phone actually encrypts to ALL .gpg-id recipients (where does store::recipients resolve the desktop cert from? earlier fredsmith-store test suggests it works — confirm).
   Also: the smith-bz store contains a directory literally named `` `pwd` `` — shell-quoting artifact from some insert path; investigate (may explain the transient phone "io error: No such file or directory" list failure).
2b. sync::clone reuse-existing-repo behavior (passero-core/src/sync.rs:31) is still a latent multi-vault footgun even though it wasn't today's bug — keep per-vault directories on the M2 list.
3. Minor UI follow-ups from reviews: poll-race guard after sign-out, timer-teardown decoupling, fingerprint persisted before keychain write (shared with generate flow), repo-list pagination >100.

## Backend contract for the UI (from Task 7)
- github_login_start → {userCode, verificationUri, interval}; github_login_poll → "pending"|"slow_down"|"authorized", throws on denial (state auto-cleared); github_logout; get_sync_settings.has_pat = keystore-derived.

## Operational notes
- Push: SSH_AUTH_SOCK=/Users/derf/.gnupg/S.gpg-agent.ssh git push (YubiKey), fallback gh HTTPS.
- cargo/rustup: export PATH="/opt/homebrew/opt/rustup/bin:$PATH". Sim build: npm run tauri ios build -- --debug --target aarch64-sim (rm gen/apple/build/passero_iOS.xcarchive + build/arm64-sim on the stale-rename error).
- Device dev run: npm run tauri ios dev -- "Fred’s iPhone" --host (positional device BEFORE --host; Personal Hotspot OFF).
- Reviewer dispatches MUST include the read-only constraint (one reviewer previously switched branches and rm -rf'd the untracked vendored plugin dir; recovered from git).
- Icon regen: `npx tauri icon app-icon.png --ios-color "#18181B"` — do not treat resulting icon diffs as anomalies.
