import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import { registerAgentIx, setPolicyIx, updatePolicyIx, setAgentActiveIx, depositSolIx,
  withdrawSolIx, transferSolIx, deriveAddresses, decodeNonce, fetchNonce, PROGRAM_ID, MAX_U64 } from '../dist/index.js';

const golden = JSON.parse(readFileSync(new URL('./abi-vectors.json', import.meta.url)));
const idl = JSON.parse(readFileSync(new URL('../src/idl/solana_guard.json', import.meta.url)));
const identity = { owner: new PublicKey(golden.owner), agent: new PublicKey(golden.agent) };
const recipient = new PublicKey(golden.recipient);
const builders = {
  register: () => registerAgentIx(identity),
  set: () => setPolicyIx(identity, 9007199254740993n, 9007199254741993n, [SystemProgram.programId, recipient]),
  update: () => updatePolicyIx(identity, { dailyLimit: 0n, allowedProtocols: [], isActive: false }),
  pause: () => setAgentActiveIx(identity, false),
  deposit: () => depositSolIx(identity, 101n),
  withdraw: () => withdrawSolIx(identity, 101n),
  transfer: () => transferSolIx(identity, recipient, 101n, 513n),
};
for (const vector of golden.vectors) {
  test(`${vector.name} binary ABI, account order and permissions match Rust/Anchor`, () => {
    const ix = builders[vector.name]();
    assert.equal(ix.programId.toBase58(), PROGRAM_ID.toBase58());
    assert.equal(ix.data.toString('hex'), vector.data);
    assert.deepEqual(ix.keys.map(k => ({ ...k, pubkey: k.pubkey.toBase58() })), vector.keys);
  });
}
test('PDAs and little-endian nonce match Rust, including alternate deployment IDs', () => {
  const addresses = deriveAddresses(identity);
  for (const name of ['config', 'policy', 'nonce', 'vault']) assert.equal(addresses[name].toBase58(), golden[name]);
  assert.equal(addresses.log(513n).toBase58(), golden.log);
  const alternate = deriveAddresses({ ...identity, programId: recipient });
  assert.notEqual(alternate.vault.toBase58(), addresses.vault.toBase58());
});
test('amounts cannot silently round, underflow, overflow, or create self-transfer receipts', () => {
  for (const amount of [0n, -1n, MAX_U64 + 1n, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => transferSolIx(identity, recipient, amount, 0n));
  }
  assert.throws(() => transferSolIx(identity, deriveAddresses(identity).vault, 1n, 0n));
  assert.throws(() => setPolicyIx(identity, 2n, 1n));
  assert.throws(() => setPolicyIx(identity, 1n, 2n, Array(11).fill(recipient)));
});
test('nonce reads reject missing, truncated, spoofed and mismatched accounts', async () => {
  const spec = idl.accounts.find(a => a.name === 'AgentNonce');
  const data = Buffer.concat([Buffer.from(spec.discriminator), identity.owner.toBuffer(), identity.agent.toBuffer(), Buffer.alloc(8), Buffer.from([255])]);
  data.writeBigUInt64LE(MAX_U64, 72);
  const account = { owner: PROGRAM_ID, data, lamports: 1, executable: false, rentEpoch: 0 };
  assert.equal(decodeNonce(account, identity), MAX_U64);
  for (const value of [null, { ...account, owner: recipient }, { ...account, data: data.subarray(0, 80) }, { ...account, data: Buffer.alloc(81) }]) {
    assert.throws(() => decodeNonce(value, identity));
  }
  assert.throws(() => decodeNonce(account, { ...identity, agent: recipient }));
  await assert.rejects(fetchNonce({ getAccountInfo: async () => { throw new Error('RPC unavailable'); } }, identity), /RPC unavailable/);
});
