//! Compiled-bytecode tests. Build first: cargo build-sbf --manifest-path programs/solana_guard/Cargo.toml
use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::instruction::Instruction,
        AccountDeserialize, AccountSerialize, InstructionData, ToAccountMetas,
    },
    litesvm::{types::FailedTransactionMetadata, LiteSVM},
    solana_guard::{AgentNonce, GuardError, Policy, TransactionLog},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const SOL: u64 = 1_000_000_000;
const SYSTEM: Pubkey = anchor_lang::system_program::ID;

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &solana_guard::id()).0
}

struct Fixture {
    svm: LiteSVM,
    owner: Keypair,
    agent: Keypair,
    recipient: Pubkey,
    config: Pubkey,
    policy: Pubkey,
    nonce: Pubkey,
    vault: Pubkey,
}

impl Fixture {
    fn new() -> Self {
        let mut svm = LiteSVM::new();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/solana_guard.so");
        let bytes = std::fs::read(&path)
            .expect("Build the program with cargo build-sbf before integration tests");
        svm.add_program(solana_guard::id(), &bytes).unwrap();
        let owner = Keypair::new();
        let agent = Keypair::new();
        let recipient = Pubkey::new_unique();
        for key in [owner.pubkey(), agent.pubkey(), recipient] {
            svm.airdrop(&key, 20 * SOL).unwrap();
        }
        let config = pda(&[
            solana_guard::AGENT_SEED,
            owner.pubkey().as_ref(),
            agent.pubkey().as_ref(),
        ]);
        let policy = pda(&[solana_guard::POLICY_SEED, config.as_ref()]);
        let nonce = pda(&[
            solana_guard::NONCE_SEED,
            owner.pubkey().as_ref(),
            agent.pubkey().as_ref(),
        ]);
        let vault = pda(&[solana_guard::VAULT_SEED, config.as_ref()]);
        let mut f = Self {
            svm,
            owner,
            agent,
            recipient,
            config,
            policy,
            nonce,
            vault,
        };
        let register = Instruction::new_with_bytes(
            solana_guard::id(),
            &solana_guard::instruction::RegisterAgent {}.data(),
            solana_guard::accounts::RegisterAgent {
                owner: f.owner.pubkey(),
                agent: f.agent.pubkey(),
                agent_config: config,
                agent_nonce: nonce,
                system_program: SYSTEM,
            }
            .to_account_metas(None),
        );
        f.owner_send(register).unwrap();
        let set = Instruction::new_with_bytes(
            solana_guard::id(),
            &solana_guard::instruction::SetPolicy {
                max_spend_per_tx: SOL,
                daily_limit: 2 * SOL,
                allowed_protocols: vec![SYSTEM],
            }
            .data(),
            solana_guard::accounts::SetPolicy {
                owner: f.owner.pubkey(),
                agent_config: config,
                policy,
                system_program: SYSTEM,
            }
            .to_account_metas(None),
        );
        f.owner_send(set).unwrap();
        let deposit = f.vault_ix(true, 5 * SOL);
        f.owner_send(deposit).unwrap();
        f
    }

    fn owner_send(&mut self, ix: Instruction) -> Result<(), Box<FailedTransactionMetadata>> {
        send(&mut self.svm, ix, &self.owner)
    }

    fn execute(&mut self, amount: u64) -> Result<(), Box<FailedTransactionMetadata>> {
        let ix = self.execute_ix(amount, SYSTEM);
        send(&mut self.svm, ix, &self.agent)
    }

    fn execute_ix(&self, amount: u64, target_protocol: Pubkey) -> Instruction {
        Instruction::new_with_bytes(
            solana_guard::id(),
            &solana_guard::instruction::ValidateAndExecute {
                amount,
                target_protocol,
            }
            .data(),
            solana_guard::accounts::ValidateAndExecute {
                agent: self.agent.pubkey(),
                owner: self.owner.pubkey(),
                agent_config: self.config,
                policy: self.policy,
                agent_nonce: self.nonce,
                tx_log: self.log_address(),
                vault: self.vault,
                recipient: self.recipient,
                system_program: SYSTEM,
            }
            .to_account_metas(None),
        )
    }

    fn vault_ix(&self, deposit: bool, amount: u64) -> Instruction {
        let data = if deposit {
            solana_guard::instruction::DepositSol { amount }.data()
        } else {
            solana_guard::instruction::WithdrawSol { amount }.data()
        };
        Instruction::new_with_bytes(
            solana_guard::id(),
            &data,
            solana_guard::accounts::ManageVault {
                owner: self.owner.pubkey(),
                agent_config: self.config,
                vault: self.vault,
                system_program: SYSTEM,
            }
            .to_account_metas(None),
        )
    }

    fn update(
        &mut self,
        max: Option<u64>,
        daily: Option<u64>,
        protocols: Option<Vec<Pubkey>>,
        active: Option<bool>,
    ) -> Result<(), Box<FailedTransactionMetadata>> {
        let ix = Instruction::new_with_bytes(
            solana_guard::id(),
            &solana_guard::instruction::UpdatePolicy {
                max_spend_per_tx: max,
                daily_limit: daily,
                allowed_protocols: protocols,
                is_active: active,
            }
            .data(),
            solana_guard::accounts::UpdatePolicy {
                owner: self.owner.pubkey(),
                agent_config: self.config,
                policy: self.policy,
            }
            .to_account_metas(None),
        );
        self.owner_send(ix)
    }

    fn pause(&mut self, paused: bool) {
        let ix = Instruction::new_with_bytes(
            solana_guard::id(),
            &solana_guard::instruction::ToggleAgent { is_active: !paused }.data(),
            solana_guard::accounts::ToggleAgent {
                owner: self.owner.pubkey(),
                agent_config: self.config,
            }
            .to_account_metas(None),
        );
        self.owner_send(ix).unwrap();
    }

    fn read<T: AccountDeserialize>(&self, address: Pubkey) -> T {
        T::try_deserialize(&mut self.svm.get_account(&address).unwrap().data.as_slice()).unwrap()
    }

    fn log_address(&self) -> Pubkey {
        let nonce: AgentNonce = self.read(self.nonce);
        pda(&[
            solana_guard::TX_LOG_SEED,
            self.config.as_ref(),
            &nonce.nonce.to_le_bytes(),
        ])
    }

    fn balance(&self, address: Pubkey) -> u64 {
        self.svm.get_balance(&address).unwrap_or(0)
    }

    fn snapshot(&self) -> (u64, u64, Vec<u8>, Vec<u8>) {
        (
            self.balance(self.vault),
            self.balance(self.recipient),
            self.svm.get_account(&self.policy).unwrap().data,
            self.svm.get_account(&self.nonce).unwrap().data,
        )
    }

    fn assert_blocked(&mut self, amount: u64, error: GuardError) {
        let before = self.snapshot();
        let log = self.log_address();
        assert_custom(self.execute(amount).unwrap_err(), error);
        assert_eq!(
            self.snapshot(),
            before,
            "rejection must not move funds or mutate counters"
        );
        assert!(
            self.svm.get_account(&log).is_none(),
            "rejection must not create an approval receipt"
        );
    }
}

fn send(
    svm: &mut LiteSVM,
    ix: Instruction,
    signer: &Keypair,
) -> Result<(), Box<FailedTransactionMetadata>> {
    svm.expire_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &svm.latest_blockhash());
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
    svm.send_transaction(tx).map(|_| ()).map_err(Box::new)
}

fn assert_custom(error: Box<FailedTransactionMetadata>, expected: GuardError) {
    let code = expected as u32 + anchor_lang::error::ERROR_CODE_OFFSET;
    assert!(
        format!("{:?}", error.err).contains(&format!("Custom({code})")),
        "unexpected error: {:?}",
        error
    );
}

#[test]
fn real_transfer_updates_balances_budget_nonce_and_recipient_receipt() {
    let mut f = Fixture::new();
    let before = f.snapshot();
    let log = f.log_address();
    f.execute(SOL / 2).unwrap();
    assert_eq!(f.balance(f.vault), before.0 - SOL / 2);
    assert_eq!(f.balance(f.recipient), before.1 + SOL / 2);
    let policy: Policy = f.read(f.policy);
    let nonce: AgentNonce = f.read(f.nonce);
    let receipt: TransactionLog = f.read(log);
    assert_eq!(policy.daily_spent, SOL / 2);
    assert_eq!(nonce.nonce, 1);
    assert_eq!(receipt.recipient, f.recipient);
    assert_eq!(receipt.amount, SOL / 2);
    assert_eq!(receipt.target_protocol, SYSTEM);
    assert_eq!(receipt.owner, f.owner.pubkey());
    assert_eq!(receipt.agent, f.agent.pubkey());
    assert_eq!(receipt.nonce, 0);
    assert!(receipt.was_approved);
}

#[test]
fn per_transaction_limit_rolls_back_everything() {
    Fixture::new().assert_blocked(SOL + 1, GuardError::ExceedsPerTxLimit);
}

#[test]
fn exact_limits_succeed_and_cumulative_spend_cannot_split_around_daily_limit() {
    let mut f = Fixture::new();
    f.execute(SOL).unwrap();
    f.execute(SOL).unwrap();
    f.assert_blocked(1, GuardError::ExceedsDailyLimit);
}

#[test]
fn kill_switch_blocks_and_unpause_restores_spending() {
    let mut f = Fixture::new();
    f.pause(true);
    f.assert_blocked(1, GuardError::AgentNotActive);
    f.pause(false);
    f.execute(1).unwrap();
}

#[test]
fn inactive_policy_fails_closed() {
    let mut f = Fixture::new();
    f.update(None, None, None, Some(false)).unwrap();
    f.assert_blocked(1, GuardError::PolicyNotActive);
}

#[test]
fn empty_allowlist_fails_closed() {
    let mut f = Fixture::new();
    f.update(None, None, Some(vec![]), None).unwrap();
    f.assert_blocked(1, GuardError::ProtocolNotAllowed);
}

#[test]
fn cannot_mislabel_system_transfer_as_another_allowlisted_protocol() {
    let mut f = Fixture::new();
    let other = Pubkey::new_unique();
    f.update(None, None, Some(vec![other]), None).unwrap();
    let before = f.snapshot();
    let ix = f.execute_ix(1, other);
    assert_custom(
        send(&mut f.svm, ix, &f.agent).unwrap_err(),
        GuardError::UnsupportedProtocol,
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn zero_amount_is_rejected() {
    Fixture::new().assert_blocked(0, GuardError::InvalidAmount);
}

#[test]
fn self_transfer_is_rejected() {
    let mut f = Fixture::new();
    f.recipient = f.vault;
    f.assert_blocked(1, GuardError::InvalidRecipient);
}

#[test]
fn insufficient_vault_balance_is_atomic() {
    let mut f = Fixture::new();
    let ix = f.vault_ix(false, 5 * SOL);
    f.owner_send(ix).unwrap();
    f.assert_blocked(1, GuardError::InsufficientVaultFunds);
}

#[test]
fn agent_cannot_spend_vault_rent_reserve() {
    let mut f = Fixture::new();
    let ix = f.vault_ix(false, 4 * SOL);
    f.owner_send(ix).unwrap();
    f.assert_blocked(SOL, GuardError::VaultRentReserve);
}

#[test]
fn failed_transfer_cpi_rolls_back_daily_window_nonce_log_and_balance() {
    let mut f = Fixture::new();
    f.execute(SOL / 2).unwrap();
    let prior: Policy = f.read(f.policy);
    let mut clock: Clock = f.svm.get_sysvar();
    clock.unix_timestamp = prior.day_start + solana_guard::SECONDS_PER_DAY;
    f.svm.set_sysvar(&clock);
    // Fault-inject a data-bearing System account. Anchor accepts its owner,
    // but the actual System Program transfer rejects data on its source.
    let mut vault = f.svm.get_account(&f.vault).unwrap();
    vault.data = vec![1];
    f.svm.set_account(f.vault, vault).unwrap();
    let before = f.snapshot();
    let log = f.log_address();
    let failure = f.execute(1).unwrap_err();
    assert!(
        failure
            .meta
            .logs
            .iter()
            .any(|log| log.contains("must not carry data")),
        "{:?}",
        failure
    );
    assert_eq!(f.snapshot(), before);
    assert!(f.svm.get_account(&log).is_none());
}

#[test]
fn rolling_window_resets_at_exact_24_hour_boundary() {
    let mut f = Fixture::new();
    f.execute(SOL).unwrap();
    f.execute(SOL).unwrap();
    let policy: Policy = f.read(f.policy);
    let mut clock: Clock = f.svm.get_sysvar();
    clock.unix_timestamp = policy.day_start + solana_guard::SECONDS_PER_DAY - 1;
    f.svm.set_sysvar(&clock);
    f.assert_blocked(1, GuardError::ExceedsDailyLimit);
    clock.unix_timestamp += 1;
    f.svm.set_sysvar(&clock);
    f.execute(SOL).unwrap();
    let reset: Policy = f.read(f.policy);
    assert_eq!(reset.daily_spent, SOL);
    assert_eq!(reset.day_start, clock.unix_timestamp);
}

#[test]
fn policy_edits_preserve_spending_history_and_invalid_edits_revert() {
    let mut f = Fixture::new();
    f.execute(SOL).unwrap();
    let prior: Policy = f.read(f.policy);
    f.update(Some(SOL / 2), Some(SOL), None, None).unwrap();
    let edited: Policy = f.read(f.policy);
    assert_eq!(edited.daily_spent, prior.daily_spent);
    assert_eq!(edited.day_start, prior.day_start);
    f.assert_blocked(1, GuardError::ExceedsDailyLimit);
    let before = f.snapshot();
    assert_custom(
        f.update(Some(2 * SOL), None, None, None).unwrap_err(),
        GuardError::InvalidDailyLimit,
    );
    assert_eq!(f.snapshot(), before);
    assert_custom(
        f.update(Some(0), None, None, None).unwrap_err(),
        GuardError::InvalidSpendingLimit,
    );
    assert_custom(
        f.update(None, None, Some(vec![SYSTEM; 11]), None)
            .unwrap_err(),
        GuardError::TooManyProtocols,
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn owner_can_recover_all_funds_while_both_gates_are_paused_then_refund() {
    let mut f = Fixture::new();
    f.pause(true);
    f.update(None, None, None, Some(false)).unwrap();
    let before = f.balance(f.owner.pubkey());
    let ix = f.vault_ix(false, 5 * SOL);
    f.owner_send(ix).unwrap();
    assert_eq!(f.balance(f.vault), 0);
    assert!(f.balance(f.owner.pubkey()) > before + 4 * SOL);
    let ix = f.vault_ix(true, SOL);
    f.owner_send(ix).unwrap();
    assert_eq!(f.balance(f.vault), SOL);
    f.assert_blocked(1, GuardError::AgentNotActive);
}

#[test]
fn wrong_owner_cannot_withdraw_or_manage_policy() {
    let mut f = Fixture::new();
    let mut ix = f.vault_ix(false, SOL);
    ix.accounts[0].pubkey = f.agent.pubkey();
    let before = f.snapshot();
    // ManageVault binds config seeds to the signer before its has_one check.
    let error = send(&mut f.svm, ix, &f.agent).unwrap_err();
    assert!(
        format!("{:?}", error.err).contains("Custom(2006)"),
        "{:?}",
        error
    );
    let mut ix = Instruction::new_with_bytes(
        solana_guard::id(),
        &solana_guard::instruction::ToggleAgent { is_active: false }.data(),
        solana_guard::accounts::ToggleAgent {
            owner: f.agent.pubkey(),
            agent_config: f.config,
        }
        .to_account_metas(None),
    );
    assert_custom(
        send(&mut f.svm, ix.clone(), &f.agent).unwrap_err(),
        GuardError::UnauthorizedOwner,
    );
    ix = Instruction::new_with_bytes(
        solana_guard::id(),
        &solana_guard::instruction::UpdatePolicy {
            max_spend_per_tx: Some(10 * SOL),
            daily_limit: Some(20 * SOL),
            allowed_protocols: None,
            is_active: None,
        }
        .data(),
        solana_guard::accounts::UpdatePolicy {
            owner: f.agent.pubkey(),
            agent_config: f.config,
            policy: f.policy,
        }
        .to_account_metas(None),
    );
    assert_custom(
        send(&mut f.svm, ix, &f.agent).unwrap_err(),
        GuardError::UnauthorizedOwner,
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn wrong_delegate_cannot_spend() {
    let mut f = Fixture::new();
    let mut ix = f.execute_ix(1, SYSTEM);
    ix.accounts[0].pubkey = f.owner.pubkey();
    let before = f.snapshot();
    assert!(send(&mut f.svm, ix, &f.owner).is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn missing_delegate_signature_is_rejected_by_anchor() {
    let mut f = Fixture::new();
    let mut ix = f.execute_ix(1, SYSTEM);
    ix.accounts[0].is_signer = false;
    let before = f.snapshot();
    let error = send(&mut f.svm, ix, &f.owner).unwrap_err();
    assert!(
        format!("{:?}", error.err).contains("Custom(3010)"),
        "{:?}",
        error
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn substituted_vault_policy_nonce_log_or_system_program_cannot_move_funds() {
    let mut f = Fixture::new();
    // Typed accounts and PDA constraints reject account substitution.
    for address in [f.vault, f.policy, f.nonce, f.log_address(), SYSTEM] {
        let mut ix = f.execute_ix(1, SYSTEM);
        let account = ix
            .accounts
            .iter_mut()
            .find(|a| a.pubkey == address)
            .unwrap();
        account.pubkey = f.recipient;
        let before = f.snapshot();
        assert!(send(&mut f.svm, ix, &f.agent).is_err());
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn direct_system_transfer_cannot_bypass_guard() {
    let mut f = Fixture::new();
    let mut ix =
        anchor_lang::solana_program::system_instruction::transfer(&f.vault, &f.recipient, SOL);
    // An attacker cannot produce a vault PDA signature; omitting it fails at CPI/runtime.
    ix.accounts[0].is_signer = false;
    let before = f.snapshot();
    assert!(send(&mut f.svm, ix, &f.agent).is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn nonce_exhaustion_fails_closed_instead_of_reusing_a_receipt() {
    let mut f = Fixture::new();
    let mut account = f.svm.get_account(&f.nonce).unwrap();
    let mut nonce: AgentNonce = f.read(f.nonce);
    nonce.nonce = u64::MAX;
    let mut data = Vec::new();
    nonce.try_serialize(&mut data).unwrap();
    account.data = data;
    f.svm.set_account(f.nonce, account).unwrap();
    f.assert_blocked(1, GuardError::NonceExhausted);
}

#[test]
fn judge_demo() {
    let mut f = Fixture::new();
    println!("SolanaGuard compiled-program demo | isolated LiteSVM | no network or real funds");
    println!("Owner funded vault: {} lamports", f.balance(f.vault));
    f.execute(SOL / 2).unwrap();
    println!(
        "PASS: 0.5 SOL reached recipient; vault balance {}",
        f.balance(f.vault)
    );
    f.assert_blocked(2 * SOL, GuardError::ExceedsPerTxLimit);
    println!("PASS: 2 SOL rejected; vault, recipient, budget, nonce unchanged");
    f.pause(true);
    f.assert_blocked(SOL / 2, GuardError::AgentNotActive);
    println!("PASS: owner kill switch blocked an otherwise valid transfer");
    let amount = f.balance(f.vault);
    let ix = f.vault_ix(false, amount);
    f.owner_send(ix).unwrap();
    println!(
        "PASS: owner recovered {} lamports while paused; vault balance 0",
        amount
    );
}

#[test]
fn stale_nonce_receipt_cannot_replay_a_transfer() {
    let mut f = Fixture::new();
    let stale = f.execute_ix(SOL / 2, SYSTEM);
    let receipt = f.log_address();
    f.execute(SOL / 2).unwrap();
    let before = f.snapshot();
    let log_data = f.svm.get_account(&receipt).unwrap().data;
    assert!(send(&mut f.svm, stale, &f.agent).is_err());
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.svm.get_account(&receipt).unwrap().data, log_data);
}

#[test]
fn valid_accounts_from_another_agent_cannot_be_substituted() {
    let mut f = Fixture::new();
    let other = Fixture::new();
    for address in [other.config, other.policy, other.nonce, other.vault] {
        f.svm
            .set_account(address, other.svm.get_account(&address).unwrap())
            .unwrap();
    }
    for (original, substitute) in [
        (f.vault, other.vault),
        (f.policy, other.policy),
        (f.nonce, other.nonce),
        (f.config, other.config),
    ] {
        let mut ix = f.execute_ix(1, SYSTEM);
        ix.accounts
            .iter_mut()
            .find(|a| a.pubkey == original)
            .unwrap()
            .pubkey = substitute;
        let before = f.snapshot();
        let other_before = f.balance(other.vault);
        assert!(send(&mut f.svm, ix, &f.agent).is_err());
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.balance(other.vault), other_before);
    }
}
