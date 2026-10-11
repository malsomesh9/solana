# SolanaGuard v0.2 audit scope

This is a proposed audit candidate, not an audit report. Freeze the exact Git commit and publish it before providing a quote request. `audit-manifest.json` contains per-file SHA-256 hashes and reproducible counts; do not quote a moving branch as a frozen release.

## Program to quote

- Program: `solana_guard`; Anchor/Rust; source identity `EdskrgG3PmMPxzaNuvp7oJjZ5MU3jkXmY4bHbSYpnsWF`.
- Production scope: `programs/solana_guard/src/lib.rs` only, including host-unit tests in that file.
- Current size: **785 physical Rust lines; 640 nonblank, non-comment-only source lines including embedded tests; approximately 585 code lines excluding embedded host tests**. Vaultline should choose its quoting convention.
- Supporting integration tests: `programs/solana_guard/tests/policy_flow.rs`.
- Supporting ABI fixture generator: `programs/solana_guard/examples/sdk_vectors.rs`.
- Client integration surface: `sdk/src/index.ts` and the source-generated IDL; quote separately if client review is wanted.
- External dependencies remain part of the trust boundary, but their implementation is excluded from the quoted source count. Anchor is pinned to 1.0.2; Cargo.lock is checked in.

The web dashboard, InsForge router, SQL migrations, authentication, hosting, and dependencies are a separate review surface. Do not describe a program-only audit as an audit of those components.

## Assets, actors, and invariants

| Invariant | Enforcement | Evidence |
| --- | --- | --- |
| Only the registered delegate can spend this vault | Signer, config identity, config/PDA binding | Wrong delegate, missing signature, direct transfer tests |
| Owner alone configures, pauses, and recovers | Owner signer and `has_one`; vault config derivation | Unauthorized owner and paused recovery tests |
| Other agents' state/funds cannot be substituted | Config, policy, nonce, log and vault PDA constraints | Valid cross-agent account substitution tests |
| Every delegated debit is the checked amount | Typed System Program CPI; target ID must equal System Program | Recipient/vault balance assertions; protocol spoof rejection |
| Agent cannot exceed per-tx or cumulative window limit | Checked addition; preserved spend history on updates | Exact limits, split spending, policy edit and expiry boundary tests |
| Rejection/CPI failure cannot consume budget or persist approval | Solana transaction rollback | Snapshots of balances/policy/nonce and receipt absence |
| A receipt identifies the actual payee and cannot be reused | Recipient stored; nonce-specific log initialization | Receipt fields, stale log replay and nonce exhaustion tests |
| Agent cannot spend vault reserve or lock owner's recovery | Agent reserve check; independent owner withdrawal | Reserve test and full paused recovery/refunding test |

## Review priorities

Review authorization and account aliasing first, especially PDA derivation, vault signer seeds, System account validation and receipt payer. Then review unit consistency, overflow/nonce exhaustion, window boundary behavior, policy edits, failed initialization/CPI rollback, and the recovery path. Check generated IDL against source and the SDK against Rust-generated instruction vectors.

A compromised agent can intentionally spend the entire allowed budget to an arbitrary System-owned recipient; this is a bounded delegation, not recipient fraud detection. An empty allowlist fails closed. Only the System Program is supported for execution; listing some other protocol does not enable that protocol.

The owner and deployment upgrade authority are trusted. On-chain pause takes effect when its transaction executes; it does not retroactively reverse confirmed payments. An attacker who controls the owner's signing key can change policy or recover funds. Off-chain wallet proofs are unrelated to the program's agent signature checks.

## Deferred functionality

SPL/Token-2022, Jupiter/DeFi routing, arbitrary CPI, recipient allowlists, oracle pricing, a true trailing-24-hour window, dashboard vault transactions, and a production deployment are excluded. Any addition of these requires a revised scope and new review. The current source change is a new candidate, not a patch proven compatible with existing deployed receipt layouts.

## Findings and remediation workflow

Before application, the owner must name a remediation contact and reserve time for triage, fixes, regression tests, and auditor re-review. Suggested process: acknowledge findings within two working days, agree on severity and exploitability, fix critical/high issues before release, and maintain a finding-to-commit/test table. This is a proposed team commitment, not an already confirmed SLA.
