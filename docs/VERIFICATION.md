# Verification evidence

Verified locally on **October 11, 2026**, on macOS arm64. Base repository commit: `dd40957` (`abhigyan1102/solana`). This document records internal development checks, not an independent security audit or a deployed-source attestation.

## Consolidated result

`PATH="$HOME/.cargo/bin:$PATH" ./scripts/verify.sh` completed with exit code **0**.

| Check | Result |
| --- | --- |
| Rust formatting | Passed |
| SBF build using locked dependencies | Passed |
| Host tests | 10 passed: 9 policy tests + Anchor program-ID check |
| LiteSVM compiled-program integration tests | 24 passed |
| Clippy | Passed with warnings denied, except the specific Anchor dispatch macro lint described below |
| Rust SDK golden-vector regeneration | Matched checked-in fixture |
| Source-generated IDL | Matched root, SDK, and app copies |
| SDK ABI / derivation / input / decoding tests | 10 passed |
| Fresh SDK/app dependency installation | Passed with scripts disabled |
| Existing dashboard TypeScript/Vite production build | Passed; repeated after the demo copy clarification |
| Browser smoke test | Homepage → read-only demo → paused-agent scenario passed; no browser console errors observed |
| Scope manifest / source hashes | Matched |
| Generated-artifact drift check | Passed |

Total: **44 passing tests**, including one integration demo that also exercises scenarios covered by other tests. This count measures executed tests, not unique security guarantees.

Clippy command: `cargo clippy --locked --all-targets -- -D warnings -A clippy::diverging_sub_expression`. Anchor 1.0.2's generated `#[program]` dispatch triggers `diverging_sub_expression` on Rust 1.94. This single macro-related lint is allowed; the code's other lint findings were fixed. The workflow is configured for GitHub Actions but has not been run on GitHub in this work.

## Compiled reviewer demo

```text
SolanaGuard compiled-program demo | isolated LiteSVM | no network or real funds
Owner funded vault: 5000000000 lamports
PASS: 0.5 SOL reached recipient; vault balance 4500000000
PASS: 2 SOL rejected; vault, recipient, budget, nonce unchanged
PASS: owner kill switch blocked an otherwise valid transfer
PASS: owner recovered 4500000000 lamports while paused; vault balance 0
```

Reproduce with `cargo test --locked --test policy_flow judge_demo -- --nocapture` after building SBF. The suite also checks exact/cumulative limits, pause/re-enable, inactive policy, empty allowlist, protocol spoofing, zero/self payments, insufficient balance, rent reserve, failed-CPI rollback across an expired spending window, window boundary, policy edits, recovery/refunding, unauthorized owner/delegate, missing signature, account substitution (including valid accounts of another agent), direct-transfer bypass, stale receipt replay, and nonce exhaustion.

Local SBF artifact SHA-256: `ffd16afb8735d82b1a52ccef52d2faf6eb9c57cf3529ae9149aa46ab52236d15`. This is the output of this local build; reproducibility across other hosts and correspondence with a deployed program have not been attested.

## Tested tools

- Rustup host Rust/Cargo: Rust 1.94.0.
- `cargo-build-sbf` 4.1.0; platform-tools v1.54; SBF Rust 1.89.0.
- Anchor CLI 0.32.1; on-chain Anchor dependency pinned at 1.0.2.
- Node 26.7.0 locally; SDK declares Node 22+ and CI is configured for Node 22.
- Python 3; source inventory needs no third-party Python packages.

## Remaining limits and dependency results

No hosted dashboard/backend mutation, wallet funding, devnet/mainnet deployment, external audit, or grant application was performed. The GitHub account connected to this environment has read access but no push access to the upstream repository. The reviewed changes must be published through an authorized repository workflow before sponsor review.

Compatible dependency updates and removal of unused frontend Anchor/BN dependencies reduced the app scanner report from **43 advisories (including one critical)** to **32 advisories (18 high, 14 moderate, no critical)**. SDK dependency scan reports **4 moderate advisories**, associated with the web3.js JSON-RPC dependency chain. Counts include transitive propagation and are not counts of confirmed exploitable application bugs. A dependency upgrade/triage remains necessary for production; breaking framework upgrades or untested overrides were not applied.

The app install also reports an inherited React Native/React peer mismatch in the mobile wallet dependency tree, and Vite warns about a bundle over 500 kB. The browser production build passed. A browser smoke test of the homepage and read-only demo passed after dependency updates; the paused-agent example visibly returned Blocked with risk 100, and the browser error log was empty. A misleading demo CTA was clarified to say evaluation dashboard. No wallet, live backend, or deployed vault flow was exercised; validate those in a deployment preview before updating the hosted app. These client surfaces are excluded from the proposed program-only audit scope.
