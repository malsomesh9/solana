# SolanaGuard SDK v0.2

Instruction builders for the native-SOL vault audit candidate. Node 22+; all lamport amounts and nonces are `bigint`. This SDK does not store keys, request wallet signatures, submit transactions, or assume that source code has been deployed.

```bash
npm ci --ignore-scripts
npm test
```

After building, import from `./dist/index.js` (or use the local package export). Example with a Connection and owner/agent wallet public keys supplied by your application:

```ts
import { Transaction } from '@solana/web3.js';
import {
  registerAgentIx, setPolicyIx, depositSolIx, fetchNonce,
  transferSolIx, setAgentActiveIx, withdrawSolIx,
} from '@solanaguard/sdk';

const identity = { owner: ownerPublicKey, agent: agentPublicKey, programId: verifiedDevnetProgramId };
// Owner signs these, using the normal wallet-adapter/send-and-confirm flow:
const setup = new Transaction().add(
  registerAgentIx(identity),
  setPolicyIx(identity, 100_000_000n, 300_000_000n),
  depositSolIx(identity, 500_000_000n),
);

// Agent signs this; it also needs SOL for transaction fees and receipt rent:
const nonce = await fetchNonce(connection, identity);
const payment = new Transaction().add(transferSolIx(identity, recipient, 50_000_000n, nonce));

// Owner signs these independently; recovery works while paused:
const pause = new Transaction().add(setAgentActiveIx(identity, false));
const recover = new Transaction().add(withdrawSolIx(identity, 450_000_000n));
```

These objects are instructions/transactions to review, not automatically sent payments. Use the actual current vault balance for full recovery and set fee payer, recent blockhash, signatures, and confirmation through your wallet/connection integration. The example's amounts assume exactly one confirmed payment and no other activity.

`fetchNonce` rejects RPC errors, missing/malformed accounts, wrong program owners and mismatched identities. It never converts a failed fetch to nonce zero. Concurrent agents must refetch the nonce after a confirmed state change. After an uncertain submission, check the original signature/receipt before retrying; retrying with a new nonce can duplicate a payment.

The source ID is a derivation default, not evidence of v0.2 deployment. Pass a verified program ID explicitly. The older `abhigyan1102/solana-guard` SDK has different seed assumptions and is not compatible.
