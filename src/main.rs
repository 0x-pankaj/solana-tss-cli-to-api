use poem::{
    IntoResponse, Response, Route, Server, get, handler, listener::TcpListener, post, web::Json,
};
use serde_json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash as SolanaHash,
    native_token,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

use crate::{
    error::Error,
    models::*,
    serialization::{AggMessage1, PartialSignature, SecretAggStepOne, Serialize},
    staking::{
        create_deactivate_stake_transaction, create_stake_account_transaction,
        create_withdraw_stake_transaction,
    },
    tss::{
        aggregate_deactivate_stake_signatures_and_broadcast,
        aggregate_stake_signatures_and_broadcast,
        aggregate_withdraw_stake_signatures_and_broadcast, deactivate_stake_step_two, key_agg,
        sign_and_broadcast, spl_sign_and_broadcast, spl_step_two, stake_step_two, step_one,
        step_two, withdraw_stake_step_two,
    },
};

use spl_token::state::{Account, Mint};

use crate::{
    models::{
        SplAggSendStepTwoRequest, SplAggSendStepTwoResponse, SplAggregateSignaturesRequest,
        SplAggregateSignaturesResponse, SplSendSingleRequest, SplSendSingleResponse,
        SplTokenBalanceRequest, SplTokenBalanceResponse,
    },
    spl_token_utils::create_spl_token_transaction,
};
use spl_associated_token_account::get_associated_token_address;
mod error;
mod models;
mod serialization;
mod spl_token_utils;
mod staking;
mod tss;

pub fn create_unsigned_transaction(
    amount: f64,
    to: &Pubkey,
    memo: Option<String>,
    payer: &Pubkey,
) -> Transaction {
    let amount = native_token::sol_to_lamports(amount);
    let transfer_ins = solana_sdk::system_instruction::transfer(payer, to, amount);
    let msg = match memo {
        None => solana_sdk::message::Message::new(&[transfer_ins], Some(payer)),
        Some(memo) => {
            let memo_ins = solana_sdk::instruction::Instruction {
                program_id: spl_memo::id(),
                accounts: Vec::new(),
                data: memo.into_bytes(),
            };
            solana_sdk::message::Message::new(&[transfer_ins, memo_ins], Some(payer))
        }
    };
    Transaction::new_unsigned(msg)
}

fn parse_keypair_bs58(s: &str) -> Result<Keypair, Error> {
    let decoded = bs58::decode(s).into_vec()?;
    Ok(Keypair::from_bytes(&decoded)?)
}

fn parse_pubkey(s: &str) -> Result<Pubkey, Error> {
    Pubkey::from_str(s).map_err(|_| {
        Error::BadBase58(bs58::decode::Error::InvalidCharacter {
            character: ' ',
            index: 0,
        })
    })
}

fn parse_hash(s: &str) -> Result<SolanaHash, Error> {
    SolanaHash::from_str(s).map_err(|_| {
        Error::BadBase58(bs58::decode::Error::InvalidCharacter {
            character: ' ',
            index: 0,
        })
    })
}

//  function to create error responses
fn error_response(error: String) -> Response {
    let error_resp = ErrorResponse { error };
    Response::builder()
        .status(poem::http::StatusCode::BAD_REQUEST)
        .content_type("application/json")
        .body(serde_json::to_string(&error_resp).unwrap_or_default())
}

//  function to create success responses
fn success_response<T: serde::Serialize>(data: T) -> Response {
    Response::builder()
        .status(poem::http::StatusCode::OK)
        .content_type("application/json")
        .body(serde_json::to_string(&data).unwrap_or_default())
}

#[handler]
async fn generate_keypair() -> impl IntoResponse {
    let keypair = Keypair::generate(&mut rand07::thread_rng());
    let response = GenerateKeypairResponse {
        secret_share: keypair.to_base58_string(),
        public_share: keypair.pubkey().to_string(),
    };
    success_response(response)
}

#[handler]
async fn balance(req: Json<BalanceRequest>) -> impl IntoResponse {
    let address = match parse_pubkey(&req.address) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let balance = match rpc_client.get_balance(&address) {
        Ok(bal) => bal,
        Err(e) => return error_response(Error::BalaceFailed(e).to_string()),
    };

    let response = BalanceResponse {
        address: address.to_string(),
        balance,
    };
    success_response(response)
}

#[handler]
async fn airdrop(req: Json<AirdropRequest>) -> impl IntoResponse {
    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let amount = native_token::sol_to_lamports(req.amount);

    let sig = match rpc_client.request_airdrop(&to, amount) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::AirdropFailed(e).to_string()),
    };

    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = AirdropResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn send_single(req: Json<SendSingleRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let mut tx = create_unsigned_transaction(req.amount, &to, req.memo.clone(), &keypair.pubkey());

    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    tx.sign(&[&keypair], recent_hash);

    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = SendSingleResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn recent_block_hash(req: Json<RecentBlockHashRequest>) -> impl IntoResponse {
    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    let response = RecentBlockHashResponse {
        recent_block_hash: recent_hash.to_string(),
    };
    success_response(response)
}

#[handler]
async fn aggregate_keys(req: Json<AggregateKeysRequest>) -> impl IntoResponse {
    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let aggkey = match key_agg(keys, None) {
        Ok(key) => key,
        Err(e) => return error_response(e.to_string()),
    };

    let aggpubkey = Pubkey::new(&*aggkey.agg_public_key.to_bytes(true));
    let response = AggregateKeysResponse {
        aggregated_public_key: aggpubkey.to_string(),
    };
    success_response(response)
}

#[handler]
async fn agg_send_step_one(req: Json<AggSendStepOneRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let (first_msg, secret) = step_one(keypair);
    let response = AggSendStepOneResponse {
        message_1: first_msg.serialize_bs58(),
        secret_state: secret.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn agg_send_step_two(req: Json<AggSendStepTwoRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let first_messages: Vec<AggMessage1> = match req
        .first_messages
        .iter()
        .map(|m| AggMessage1::deserialize_bs58(m))
        .collect::<Result<_, _>>()
    {
        Ok(msgs) => msgs,
        Err(e) => return error_response(e.to_string()),
    };

    let secret_state = match SecretAggStepOne::deserialize_bs58(&req.secret_state) {
        Ok(state) => state,
        Err(e) => return error_response(e.to_string()),
    };

    let sig = match step_two(
        keypair,
        req.amount,
        to,
        req.memo.clone(),
        block_hash,
        keys,
        first_messages,
        secret_state,
    ) {
        Ok(signature) => signature,
        Err(e) => return error_response(e.to_string()),
    };

    let response = AggSendStepTwoResponse {
        partial_signature: sig.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn aggregate_signatures(req: Json<AggregateSignaturesRequest>) -> impl IntoResponse {
    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let signatures: Vec<PartialSignature> = match req
        .signatures
        .iter()
        .map(|s| PartialSignature::deserialize_bs58(s))
        .collect::<Result<_, _>>()
    {
        Ok(sigs) => sigs,
        Err(e) => return error_response(e.to_string()),
    };

    let tx = match sign_and_broadcast(
        req.amount,
        to,
        req.memo.clone(),
        block_hash,
        keys,
        signatures,
    ) {
        Ok(transaction) => transaction,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &block_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = AggregateSignaturesResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

//////////////////////// spl /////////////////////////////

// token_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
// 6A2GHg17A2YUbLp7qma1pbvnS7deav7Tq3tthQHa8zt5
#[handler]
async fn spl_token_balance(req: Json<SplTokenBalanceRequest>) -> impl IntoResponse {
    let owner = match parse_pubkey(&req.owner) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let token_mint = match parse_pubkey(&req.token_mint) {
        Ok(mint) => mint,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());

    // Get the associated token address
    let token_account = get_associated_token_address(&owner, &token_mint);

    // Get token account info
    let account_info = match rpc_client.get_account(&token_account) {
        Ok(account) => account,
        Err(_) => return error_response("Token account not found".to_string()),
    };

    // Parse the token account data
    let token_account_data = match Account::unpack(&account_info.data) {
        Ok(data) => data,
        Err(e) => return error_response(format!("Failed to parse token account: {}", e)),
    };

    // Get mint info to get decimals
    let mint_info = match rpc_client.get_account(&token_mint) {
        Ok(account) => account,
        Err(_) => return error_response("Token mint not found".to_string()),
    };

    let mint_data = match Mint::unpack(&mint_info.data) {
        Ok(data) => data,
        Err(e) => return error_response(format!("Failed to parse mint account: {}", e)),
    };

    let response = SplTokenBalanceResponse {
        owner: owner.to_string(),
        token_mint: token_mint.to_string(),
        balance: token_account_data.amount,
        decimals: mint_data.decimals,
    };
    success_response(response)
}

#[handler]
async fn spl_send_single(req: Json<SplSendSingleRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let token_mint = match parse_pubkey(&req.token_mint) {
        Ok(mint) => mint,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());

    // Convert amount to proper token units
    let token_amount = (req.amount * 10_f64.powi(req.decimals as i32)) as u64;

    //Derive ATAs
    let from_ata =
        spl_associated_token_account::get_associated_token_address(&keypair.pubkey(), &token_mint);
    let to_ata = spl_associated_token_account::get_associated_token_address(&to, &token_mint);

    //checking if destination ATA exists
    let to_ata_exists = match rpc_client.get_account(&to_ata) {
        Ok(_) => true,
        Err(_) => false,
    };

    let mut instructions = vec![];

    // Create destination ATA if it doesn't exist
    if !to_ata_exists {
        let create_ata_instruction =
            spl_associated_token_account::instruction::create_associated_token_account(
                &keypair.pubkey(), // Payer
                &to,               // Owner
                &token_mint,       // Mint
                &spl_token::id(),  // Token program
            );
        instructions.push(create_ata_instruction);
    }

    // Create transfer instruction
    let transfer_instruction = match spl_token::instruction::transfer(
        &spl_token::id(),
        &from_ata,
        &to_ata,
        &keypair.pubkey(),
        &[],
        token_amount,
    ) {
        Ok(instr) => instr,
        Err(e) => return error_response(e.to_string()),
    };

    instructions.push(transfer_instruction);

    // Add memo if provided
    if let Some(memo) = req.memo.clone() {
        instructions.push(spl_memo::build_memo(memo.as_bytes(), &[]));
    }

    // Create and sign transaction
    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    let mut tx = Transaction::new_with_payer(&instructions, Some(&keypair.pubkey()));
    tx.sign(&[&keypair], recent_hash);

    // Send transaction
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => {
            if let Some(rpc_err) = e.get_transaction_error() {
                eprintln!("Transaction error details: {:?}", rpc_err);
            }
            return error_response(Error::SendTransactionFailed(e).to_string());
        }
    };

    // Confirm transaction
    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = SplSendSingleResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn spl_agg_send_step_two(req: Json<SplAggSendStepTwoRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let token_mint = match parse_pubkey(&req.token_mint) {
        Ok(mint) => mint,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let first_messages: Vec<AggMessage1> = match req
        .first_messages
        .iter()
        .map(|m| AggMessage1::deserialize_bs58(m))
        .collect::<Result<_, _>>()
    {
        Ok(msgs) => msgs,
        Err(e) => return error_response(e.to_string()),
    };

    let secret_state = match SecretAggStepOne::deserialize_bs58(&req.secret_state) {
        Ok(state) => state,
        Err(e) => return error_response(e.to_string()),
    };

    let sig = match spl_step_two(
        keypair,
        req.amount,
        to,
        token_mint,
        req.decimals,
        req.memo.clone(),
        block_hash,
        keys,
        first_messages,
        secret_state,
    ) {
        Ok(signature) => signature,
        Err(e) => return error_response(e.to_string()),
    };

    let response = SplAggSendStepTwoResponse {
        partial_signature: sig.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn spl_aggregate_signatures(req: Json<SplAggregateSignaturesRequest>) -> impl IntoResponse {
    let to = match parse_pubkey(&req.to) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let token_mint = match parse_pubkey(&req.token_mint) {
        Ok(mint) => mint,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let signatures: Vec<PartialSignature> = match req
        .signatures
        .iter()
        .map(|s| PartialSignature::deserialize_bs58(s))
        .collect::<Result<_, _>>()
    {
        Ok(sigs) => sigs,
        Err(e) => return error_response(e.to_string()),
    };

    let tx = match spl_sign_and_broadcast(
        req.amount,
        to,
        token_mint,
        req.decimals,
        req.memo.clone(),
        block_hash,
        keys,
        signatures,
    ) {
        Ok(transaction) => transaction,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &block_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = SplAggregateSignaturesResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

// -------------------------- staking -----------------------//
//

#[handler]
async fn stake_account(req: Json<StakeAccountRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let vote_account = match parse_pubkey(&req.validator_vote_accont) {
        Ok(vc) => vc,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let mut tx = match create_stake_account_transaction(
        req.stake_amount,
        &req.seed,
        &keypair.pubkey(),
        &vote_account,
    ) {
        Ok(tx) => tx,
        Err(e) => return error_response(e.to_string()),
    };

    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    tx.sign(&[&keypair], recent_hash);

    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let stake_account = match Pubkey::create_with_seed(
        &keypair.pubkey(),
        &req.seed,
        &solana_sdk::stake::program::id(),
    ) {
        Ok(addr) => addr,
        Err(_) => {
            return error_response("Invalid stake account seed".to_string());
        }
    };

    let response = StakeAccountResponse {
        stake_account_address: stake_account.to_string(),
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn deactivate_stake(req: Json<DeactivateStakeRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let mut tx = create_deactivate_stake_transaction(&stake_accountt, &keypair.pubkey());

    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    tx.sign(&[&keypair], recent_hash);

    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = DeactivateStakeResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn withdraw_stake(req: Json<WithdrawStakeRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let destination = match parse_pubkey(&req.destination) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let mut tx = create_withdraw_stake_transaction(
        &stake_accountt,
        &destination,
        &keypair.pubkey(),
        req.amount,
    );

    let recent_hash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => return error_response(Error::RecentHashFailed(e).to_string()),
    };

    tx.sign(&[&keypair], recent_hash);

    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = WithdrawStakeResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn agg_stake_step_two(req: Json<AggStakeStepTwoRequest>) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let vote_account = match parse_pubkey(&req.validator_vote_accont) {
        Ok(vc) => vc,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let first_messages: Vec<AggMessage1> = match req
        .first_messages
        .iter()
        .map(|m| AggMessage1::deserialize_bs58(m))
        .collect::<Result<_, _>>()
    {
        Ok(msgs) => msgs,
        Err(e) => return error_response(e.to_string()),
    };

    let secret_state = match SecretAggStepOne::deserialize_bs58(&req.secret_state) {
        Ok(state) => state,
        Err(e) => return error_response(e.to_string()),
    };

    let sig = match stake_step_two(
        keypair,
        req.stake_amount,
        req.seed.clone(),
        vote_account,
        block_hash,
        keys,
        first_messages,
        secret_state,
    ) {
        Ok(signature) => signature,
        Err(e) => return error_response(e.to_string()),
    };

    let response = AggStakeStepTwoResponse {
        partial_signature: sig.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn agg_deactivate_stake_step_two(
    req: Json<AggDeactivateStakeStepTwoRequest>,
) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let first_messages: Vec<AggMessage1> = match req
        .first_messages
        .iter()
        .map(|m| AggMessage1::deserialize_bs58(m))
        .collect::<Result<_, _>>()
    {
        Ok(msgs) => msgs,
        Err(e) => return error_response(e.to_string()),
    };

    let secret_state = match SecretAggStepOne::deserialize_bs58(&req.secret_state) {
        Ok(state) => state,
        Err(e) => return error_response(e.to_string()),
    };

    let sig = match deactivate_stake_step_two(
        keypair,
        stake_accountt,
        block_hash,
        keys,
        first_messages,
        secret_state,
    ) {
        Ok(signature) => signature,
        Err(e) => return error_response(e.to_string()),
    };

    let response = AggDeactivateStakeStepTwoResponse {
        partial_signature: sig.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn agg_withdraw_stake_step_two(
    req: Json<AggWithdrawStakeStepTwoRequest>,
) -> impl IntoResponse {
    let keypair = match parse_keypair_bs58(&req.keypair) {
        Ok(kp) => kp,
        Err(e) => return error_response(e.to_string()),
    };

    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let destination = match parse_pubkey(&req.destination) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let first_messages: Vec<AggMessage1> = match req
        .first_messages
        .iter()
        .map(|m| AggMessage1::deserialize_bs58(m))
        .collect::<Result<_, _>>()
    {
        Ok(msgs) => msgs,
        Err(e) => return error_response(e.to_string()),
    };

    let secret_state = match SecretAggStepOne::deserialize_bs58(&req.secret_state) {
        Ok(state) => state,
        Err(e) => return error_response(e.to_string()),
    };

    let sig = match withdraw_stake_step_two(
        keypair,
        stake_accountt,
        destination,
        req.amount,
        block_hash,
        keys,
        first_messages,
        secret_state,
    ) {
        Ok(signature) => signature,
        Err(e) => return error_response(e.to_string()),
    };

    let response = AggWithdrawStakeStepTwoResponse {
        partial_signature: sig.serialize_bs58(),
    };
    success_response(response)
}

#[handler]
async fn aggregate_stake_signatures(
    req: Json<AggregateStakeSignaturesRequest>,
) -> impl IntoResponse {
    let vote_account = match parse_pubkey(&req.validator_vote_accont) {
        Ok(vc) => vc,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let signatures: Vec<PartialSignature> = match req
        .signatures
        .iter()
        .map(|s| PartialSignature::deserialize_bs58(s))
        .collect::<Result<_, _>>()
    {
        Ok(sigs) => sigs,
        Err(e) => return error_response(e.to_string()),
    };

    let tx = match aggregate_stake_signatures_and_broadcast(
        req.stake_amount,
        req.seed.clone(),
        vote_account,
        block_hash,
        keys,
        signatures,
    ) {
        Ok(tx) => tx,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &block_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = AggregateStakeSignaturesResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn aggregate_deactivate_stake_signatures(
    req: Json<AggregateDeactivateStakeSignaturesRequest>,
) -> impl IntoResponse {
    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let signatures: Vec<PartialSignature> = match req
        .signatures
        .iter()
        .map(|s| PartialSignature::deserialize_bs58(s))
        .collect::<Result<_, _>>()
    {
        Ok(sigs) => sigs,
        Err(e) => return error_response(e.to_string()),
    };

    let tx = match aggregate_deactivate_stake_signatures_and_broadcast(
        stake_accountt,
        block_hash,
        keys,
        signatures,
    ) {
        Ok(tx) => tx,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &block_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = AggregateDeactivateStakeSignaturesResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

#[handler]
async fn aggregate_withdraw_stake_signatures(
    req: Json<AggregateWithdrawStakeSignaturesRequest>,
) -> impl IntoResponse {
    let stake_accountt = match parse_pubkey(&req.stake_account) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let destination = match parse_pubkey(&req.destination) {
        Ok(addr) => addr,
        Err(e) => return error_response(e.to_string()),
    };

    let block_hash = match parse_hash(&req.recent_block_hash) {
        Ok(hash) => hash,
        Err(e) => return error_response(e.to_string()),
    };

    let keys: Vec<Pubkey> = match req
        .keys
        .iter()
        .map(|k| parse_pubkey(k))
        .collect::<Result<_, _>>()
    {
        Ok(keys) => keys,
        Err(e) => return error_response(e.to_string()),
    };

    let signatures: Vec<PartialSignature> = match req
        .signatures
        .iter()
        .map(|s| PartialSignature::deserialize_bs58(s))
        .collect::<Result<_, _>>()
    {
        Ok(sigs) => sigs,
        Err(e) => return error_response(e.to_string()),
    };

    let tx = match aggregate_withdraw_stake_signatures_and_broadcast(
        stake_accountt,
        destination,
        req.amount,
        block_hash,
        keys,
        signatures,
    ) {
        Ok(tx) => tx,
        Err(e) => return error_response(e.to_string()),
    };

    let rpc_client = RpcClient::new(req.net.get_cluster_url().to_string());
    let sig = match rpc_client.send_transaction(&tx) {
        Ok(signature) => signature,
        Err(e) => return error_response(Error::SendTransactionFailed(e).to_string()),
    };

    if let Err(e) =
        rpc_client.confirm_transaction_with_spinner(&sig, &block_hash, rpc_client.commitment())
    {
        return error_response(Error::ConfirmingTransactionFailed(e).to_string());
    }

    let response = AggregateWithdrawStakeSignaturesResponse {
        transaction_id: sig.to_string(),
    };
    success_response(response)
}

//staking end her

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Route::new()
        .at("/api/generate", get(generate_keypair))
        .at("/api/balance", post(balance))
        .at("/api/airdrop", post(airdrop))
        .at("/api/send_single", post(send_single))
        .at("/api/recent_block_hash", post(recent_block_hash))
        .at("/api/aggregate_keys", post(aggregate_keys))
        .at("/api/agg_send_step_one", post(agg_send_step_one))
        .at("/api/agg_send_step_two", post(agg_send_step_two))
        .at("/api/aggregate_signatures", post(aggregate_signatures))
        .at("/api/spl_token_balance", post(spl_token_balance))
        .at("/api/spl_send_single", post(spl_send_single))
        .at("/api/spl_agg_send_step_two", post(spl_agg_send_step_two))
        .at(
            "/api/spl_aggregate_signatures",
            post(spl_aggregate_signatures),
        )
        .at("/api/stake", post(stake_account))
        .at("/api/deactivate_stake", post(deactivate_stake))
        .at("/api/withdraw_stake", post(withdraw_stake))
        .at("/api/agg_stake_step_two", post(agg_stake_step_two))
        .at(
            "/api/agg_deactivate_stake_step_two",
            post(agg_deactivate_stake_step_two),
        )
        .at(
            "/api/agg_withdraw_stake_step_two",
            post(agg_withdraw_stake_step_two),
        )
        .at(
            "/api/aggregate_stake_signatures",
            post(aggregate_stake_signatures),
        )
        .at(
            "/api/aggregate_deactivate_stake_signatures",
            post(aggregate_deactivate_stake_signatures),
        )
        .at(
            "/api/aggregate_withdraw_stake_signatures",
            post(aggregate_withdraw_stake_signatures),
        );

    Server::new(TcpListener::bind("127.0.0.1:8000"))
        .run(app)
        .await?;

    Ok(())
}
