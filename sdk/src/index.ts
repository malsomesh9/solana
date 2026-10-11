import { Buffer } from 'buffer';
import { AccountInfo, Connection, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import idl from './idl/solana_guard.json' with { type: 'json' };

/** Source program ID, not a claim that v0.2 is deployed at this address. */
export const PROGRAM_ID = new PublicKey(idl.address);
export const MAX_U64 = (1n << 64n) - 1n;
export type AgentIdentity = { owner: PublicKey; agent: PublicKey; programId?: PublicKey };

function u64(value: bigint): Buffer {
  if (typeof value !== 'bigint' || value < 0n || value > MAX_U64) throw new RangeError('Expected bigint in u64 range');
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64LE(value);
  return bytes;
}
function amountBytes(value: bigint): Buffer {
  if (value === 0n) throw new RangeError('Amount must be positive');
  return u64(value);
}
function protocolsBytes(protocols: readonly PublicKey[]): Buffer {
  if (protocols.length > 10) throw new RangeError('At most 10 protocols');
  const count = Buffer.alloc(4);
  count.writeUInt32LE(protocols.length);
  return Buffer.concat([count, ...protocols.map(p => p.toBuffer())]);
}
function option<T>(value: T | undefined, encode: (value: T) => Buffer): Buffer {
  return value === undefined ? Buffer.from([0]) : Buffer.concat([Buffer.from([1]), encode(value)]);
}

/** Seeds are from programs/solana_guard, not the incompatible legacy repo. */
export function deriveAddresses({ owner, agent, programId = PROGRAM_ID }: AgentIdentity) {
  const derive = (...seeds: Buffer[]) => PublicKey.findProgramAddressSync(seeds, programId)[0];
  const config = derive(Buffer.from('agent_config'), owner.toBuffer(), agent.toBuffer());
  return {
    config,
    policy: derive(Buffer.from('policy'), config.toBuffer()),
    nonce: derive(Buffer.from('agent_nonce'), owner.toBuffer(), agent.toBuffer()),
    vault: derive(Buffer.from('vault'), config.toBuffer()),
    log: (nonce: bigint) => derive(Buffer.from('tx_log'), config.toBuffer(), u64(nonce)),
  };
}

function instruction(identity: AgentIdentity, name: string, accounts: Record<string, PublicKey>, args: Buffer = Buffer.alloc(0)) {
  const spec = idl.instructions.find(i => i.name === name);
  if (!spec) throw new Error(`Unknown IDL instruction ${name}`);
  return new TransactionInstruction({
    programId: identity.programId ?? PROGRAM_ID,
    keys: spec.accounts.map(a => {
      const address = 'address' in a ? a.address : undefined;
      const pubkey = address ? new PublicKey(address) : accounts[a.name];
      if (!pubkey) throw new Error(`Missing ${a.name}`);
      return { pubkey, isSigner: 'signer' in a && a.signer === true, isWritable: 'writable' in a && a.writable === true };
    }),
    data: Buffer.concat([Buffer.from(spec.discriminator), args]),
  });
}
function accountsFor(identity: AgentIdentity) {
  const pdas = deriveAddresses(identity);
  return { owner: identity.owner, agent: identity.agent, agent_config: pdas.config,
    agent_nonce: pdas.nonce, policy: pdas.policy, vault: pdas.vault };
}

/** Owner signs and pays registration rent. Agent public key need not sign. */
export function registerAgentIx(identity: AgentIdentity) {
  return instruction(identity, 'register_agent', accountsFor(identity));
}
export function setPolicyIx(identity: AgentIdentity, maxSpendPerTx: bigint, dailyLimit: bigint, allowedProtocols: readonly PublicKey[] = [SystemProgram.programId]) {
  if (dailyLimit < maxSpendPerTx) throw new RangeError('Daily limit must cover per-transaction limit');
  return instruction(identity, 'set_policy', accountsFor(identity), Buffer.concat([
    amountBytes(maxSpendPerTx), u64(dailyLimit), protocolsBytes(allowedProtocols),
  ]));
}
export function updatePolicyIx(identity: AgentIdentity, patch: {
  maxSpendPerTx?: bigint; dailyLimit?: bigint; allowedProtocols?: readonly PublicKey[]; isActive?: boolean;
}) {
  return instruction(identity, 'update_policy', accountsFor(identity), Buffer.concat([
    option(patch.maxSpendPerTx, amountBytes), option(patch.dailyLimit, u64),
    option(patch.allowedProtocols, protocolsBytes), option(patch.isActive, v => Buffer.from([Number(v)])),
  ]));
}
export function setAgentActiveIx(identity: AgentIdentity, active: boolean) {
  return instruction(identity, 'toggle_agent', accountsFor(identity), Buffer.from([Number(active)]));
}
export function depositSolIx(identity: AgentIdentity, lamports: bigint) {
  return instruction(identity, 'deposit_sol', accountsFor(identity), amountBytes(lamports));
}
export function withdrawSolIx(identity: AgentIdentity, lamports: bigint) {
  return instruction(identity, 'withdraw_sol', accountsFor(identity), amountBytes(lamports));
}

/** Agent signs. Fetch current nonce first. A stale nonce fails without moving funds.
 * Only native SOL; there is no arbitrary program/data/account forwarding. */
export function transferSolIx(identity: AgentIdentity, recipient: PublicKey, lamports: bigint, nonce: bigint) {
  const pdas = deriveAddresses(identity);
  if (recipient.equals(pdas.vault)) throw new Error('Recipient cannot be the vault');
  return instruction(identity, 'validate_and_execute', {
    ...accountsFor(identity), recipient, tx_log: pdas.log(nonce),
  }, Buffer.concat([amountBytes(lamports), SystemProgram.programId.toBuffer()]));
}

/** RPC/decoding failures surface as errors, never as nonce zero. */
export async function fetchNonce(connection: Connection, identity: AgentIdentity): Promise<bigint> {
  return decodeNonce(await connection.getAccountInfo(deriveAddresses(identity).nonce, 'confirmed'), identity);
}
export function decodeNonce(account: AccountInfo<Buffer> | null, identity: AgentIdentity): bigint {
  if (!account || !account.owner.equals(identity.programId ?? PROGRAM_ID)) throw new Error('Missing nonce or wrong program owner');
  const spec = idl.accounts.find(a => a.name === 'AgentNonce');
  if (!spec || account.data.length !== 81 || !account.data.subarray(0, 8).equals(Buffer.from(spec.discriminator))) throw new Error('Invalid AgentNonce account');
  if (!new PublicKey(account.data.subarray(8, 40)).equals(identity.owner) ||
      !new PublicKey(account.data.subarray(40, 72)).equals(identity.agent)) throw new Error('Nonce identity mismatch');
  return account.data.readBigUInt64LE(72);
}
