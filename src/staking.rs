use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    stake::{
        instruction as stake_instruction,
        state::{Authorized, Lockup, StakeStateV2},
    },
    system_instruction,
    transaction::Transaction,
};

use crate::error::Error;

pub fn create_stake_account_transaction(
    stake_amount: u64,
    seed: &str,
    payer: &Pubkey,
    validator_vote_accont: &Pubkey,
) -> Result<Transaction, Error> {
    let stake_account = Pubkey::create_with_seed(payer, seed, &solana_sdk::stake::program::id())
        .map_err(|e| Error::InvalidStakeAccountSeed(e.to_string()))?;

    let space = std::mem::size_of::<StakeStateV2>() as u64;
    let rent = RpcClient::new("https://api.testnet.solana.com")
        .get_minimum_balance_for_rent_exemption(space as usize)
        .map_err(|e| Error::StakeAccountCreationFailed(e.to_string()))?;

    let create_account_ins = system_instruction::create_account_with_seed(
        payer,
        &stake_account,
        payer,
        seed,
        rent + stake_amount,
        space,
        &solana_sdk::stake::program::id(),
    );

    let initialize_ins = stake_instruction::initialize(
        &stake_account,
        &Authorized {
            staker: *payer,
            withdrawer: *payer,
        },
        &Lockup::default(),
    );

    let delegate_ins =
        stake_instruction::delegate_stake(&stake_account, payer, validator_vote_accont);

    let msg = solana_sdk::message::Message::new(
        &[create_account_ins, initialize_ins, delegate_ins],
        Some(payer),
    );

    Ok(Transaction::new_unsigned(msg))
}

pub fn create_deactivate_stake_transaction(
    stake_account: &Pubkey,
    authorized: &Pubkey,
) -> Transaction {
    let deactivate_ins = stake_instruction::deactivate_stake(stake_account, authorized);
    let msg = solana_sdk::message::Message::new(&[deactivate_ins], Some(authorized));
    Transaction::new_unsigned(msg)
}

pub fn create_withdraw_stake_transaction(
    stake_account: &Pubkey,
    destination: &Pubkey,
    authorized: &Pubkey,
    amount: u64,
) -> Transaction {
    let withdraw_ins =
        stake_instruction::withdraw(stake_account, authorized, destination, amount, None);
    let msg = solana_sdk::message::Message::new(&[withdraw_ins], Some(authorized));
    Transaction::new_unsigned(msg)
}
