# Deploying the v0.2 candidate

No deployment was performed in this work. Do not send funds to the previously advertised devnet address expecting the new vault instructions to exist there. Do not use mainnet for the audit-candidate demo.

## Compatibility

`validate_and_execute` now requires a vault and recipient and executes a SOL payment. Its account list changed. `TransactionLog` now includes the recipient and has a different layout/size. Config, policy, and nonce layouts are unchanged. Existing logs cannot be decoded as v0.2 receipts without a version-aware migration/reader. The generated v0.2 IDL is incompatible with the stale legacy IDL in the original frontend checkout.

The source declares the existing program ID to keep derivation review consistent. Deploying an upgrade at that address requires the actual upgrade authority, explicit deployment authorization, and a reviewed migration plan. A fresh devnet deployment under a new ID is preferable for testing this candidate; update `declare_id!`, Anchor.toml and all generated IDL/client artifacts together, then rebuild and rerun verification.

## Controlled rollout

1. Freeze and publish a review commit. Keep the program keypair and payer outside Git; generate/use them in your own development environment.
2. Choose a fresh devnet program address or document authority and account migration for an upgrade. Configure the address consistently. Use `anchor keys sync` only after checking which keypair/address is intended; it can change source IDs.
3. Build SBF, regenerate IDL, regenerate Rust SDK vectors, run verification, regenerate the scope manifest, and commit the result. A changed program ID changes PDA vectors and file hashes.
4. Deploy on devnet with an explicitly selected cluster and payer. Record the deployment transaction signature, program-data address, upgrade authority, slot, and source commit. Never imply that an explorer address alone proves source correspondence.
5. With a fresh owner/agent pair and small test SOL amounts, register, set policy, deposit, transfer, verify both balances and receipt, attempt an over-limit transaction, pause, attempt a valid transaction, and recover funds. Save signatures and observed balances in a deployment-evidence file.
6. Keep the hosted dashboard labelled as off-chain evaluation until it actually signs/submits the new instructions. Its SQL pause does not pause an on-chain config.
7. External audit, finding remediation, re-review, authority custody, monitoring, and release decisions precede any production funds.

## Operational behavior

The agent needs its own SOL for transaction fees and successful receipt rent; those expenses are outside the vault spending policy. Funding the vault alone does not fund the agent's fee payer. Rejected transactions can still incur network fees. Each successful payment leaves a rent-funded receipt account; there is no close/reclaim instruction in this scope.

A zero-balance vault can disappear from ledger storage and can be funded again at the same PDA. A nonzero partial withdrawal must satisfy runtime rent rules; withdraw the full balance when recovering everything. Agent payments keep a zero-data rent reserve. Keep RPC commitments and transaction confirmation explicit; after an uncertain submission, inspect the receipt/transaction before retrying a payment. A stale nonce fails, but an automated retry with a new nonce can create a second payment if the first already succeeded.
