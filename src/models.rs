use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Network {
    pub fn get_cluster_url(&self) -> &'static str {
        match self {
            Self::Mainnet => "https://api.mainnet-beta.solana.com",
            Self::Testnet => "https://api.testnet.solana.com",
            Self::Devnet => "https://api.devnet.solana.com",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateKeypairResponse {
    pub secret_share: String,
    pub public_share: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceRequest {
    pub address: String,
    pub net: Network,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub address: String,
    pub balance: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirdropRequest {
    pub to: String,
    pub amount: f64,
    pub net: Network,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirdropResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendSingleRequest {
    pub keypair: String,
    pub amount: f64,
    pub to: String,
    pub net: Network,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendSingleResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentBlockHashRequest {
    pub net: Network,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentBlockHashResponse {
    pub recent_block_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateKeysRequest {
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateKeysResponse {
    pub aggregated_public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStepOneRequest {
    pub keypair: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStepOneResponse {
    pub message_1: String,
    pub secret_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStepTwoRequest {
    pub keypair: String,
    pub amount: f64,
    pub to: String,
    pub memo: Option<String>,
    pub recent_block_hash: String,
    pub keys: Vec<String>,
    pub first_messages: Vec<String>,
    pub secret_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStepTwoResponse {
    pub partial_signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateSignaturesRequest {
    pub signatures: Vec<String>,
    pub amount: f64,
    pub to: String,
    pub memo: Option<String>,
    pub recent_block_hash: String,
    pub net: Network,
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateSignaturesResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// SPL Token Transfer Models
#[derive(Debug, Serialize, Deserialize)]
pub struct SplTokenBalanceRequest {
    pub owner: String,
    pub token_mint: String,
    pub net: Network,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplTokenBalanceResponse {
    pub owner: String,
    pub token_mint: String,
    pub balance: u64,
    pub decimals: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplSendSingleRequest {
    pub keypair: String,
    pub amount: f64,
    pub to: String,
    pub token_mint: String,
    pub decimals: u8,
    pub net: Network,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplSendSingleResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplAggSendStepTwoRequest {
    pub keypair: String,
    pub amount: f64,
    pub to: String,
    pub token_mint: String,
    pub decimals: u8,
    pub memo: Option<String>,
    pub recent_block_hash: String,
    pub keys: Vec<String>,
    pub first_messages: Vec<String>,
    pub secret_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplAggSendStepTwoResponse {
    pub partial_signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplAggregateSignaturesRequest {
    pub signatures: Vec<String>,
    pub amount: f64,
    pub to: String,
    pub token_mint: String,
    pub decimals: u8,
    pub memo: Option<String>,
    pub recent_block_hash: String,
    pub net: Network,
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SplAggregateSignaturesResponse {
    pub transaction_id: String,
}

//-----------------------stake Account Creation

#[derive(Debug, Serialize, Deserialize)]
pub struct StakeAccountRequest {
    pub net: Network,
    pub keypair: String,   // Base58 encoded keypair
    pub stake_amount: u64, // Amount to stake in lamports
    pub seed: String,      // Seed for deriving the stake account
    pub validator_vote_accont: String,
}

#[derive(Debug, Serialize)]
pub struct StakeAccountResponse {
    pub stake_account_address: String,
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeactivateStakeRequest {
    pub net: Network,
    pub keypair: String,       // Base58 encoded keypair
    pub stake_account: String, // Stake account pubkey
}

#[derive(Debug, Serialize)]
pub struct DeactivateStakeResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawStakeRequest {
    pub net: Network,
    pub keypair: String,       // Base58 encoded keypair
    pub stake_account: String, // Stake account pubkey
    pub destination: String,   // Destination pubkey for withdrawn funds
    pub amount: u64,           // Amount to withdraw in lamports
}

#[derive(Debug, Serialize)]
pub struct WithdrawStakeResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggStakeStepOneRequest {
    pub keypair: String, // Base58 encoded keypair
}

#[derive(Debug, Serialize)]
pub struct AggStakeStepOneResponse {
    pub message_1: String,    // Base58 encoded AggMessage1
    pub secret_state: String, // Base58 encoded SecretAggStepOne
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggStakeStepTwoRequest {
    pub net: Network,
    pub keypair: String,   // Base58 encoded keypair
    pub stake_amount: u64, // Amount to stake in lamports
    pub seed: String,      // Seed for stake account
    pub validator_vote_accont: String,
    pub keys: Vec<String>,           // List of pubkeys for aggregation
    pub first_messages: Vec<String>, // Base58 encoded AggMessage1
    pub secret_state: String,        // Base58 encoded SecretAggStepOne from step one
    pub recent_block_hash: String,   // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggStakeStepTwoResponse {
    pub partial_signature: String, // Base58 encoded PartialSignature
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggDeactivateStakeStepOneRequest {
    pub keypair: String, // Base58 encoded keypair
}

#[derive(Debug, Serialize)]
pub struct AggDeactivateStakeStepOneResponse {
    pub message_1: String,    // Base58 encoded AggMessage1
    pub secret_state: String, // Base58 encoded SecretAggStepOne
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggDeactivateStakeStepTwoRequest {
    pub net: Network,
    pub keypair: String,             // Base58 encoded keypair
    pub stake_account: String,       // Stake account pubkey
    pub keys: Vec<String>,           // List of pubkeys for aggregation
    pub first_messages: Vec<String>, // Base58 encoded AggMessage1
    pub secret_state: String,        // Base58 encoded SecretAggStepOne from step one
    pub recent_block_hash: String,   // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggDeactivateStakeStepTwoResponse {
    pub partial_signature: String, // Base58 encoded PartialSignature
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggWithdrawStakeStepOneRequest {
    pub keypair: String, // Base58 encoded keypair
}

#[derive(Debug, Serialize)]
pub struct AggWithdrawStakeStepOneResponse {
    pub message_1: String,    // Base58 encoded AggMessage1
    pub secret_state: String, // Base58 encoded SecretAggStepOne
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggWithdrawStakeStepTwoRequest {
    pub net: Network,
    pub keypair: String,             // Base58 encoded keypair
    pub stake_account: String,       // Stake account pubkey
    pub destination: String,         // Destination pubkey for withdrawn funds
    pub amount: u64,                 // Amount to withdraw in lamports
    pub keys: Vec<String>,           // List of pubkeys for aggregation
    pub first_messages: Vec<String>, // Base58 encoded AggMessage1
    pub secret_state: String,        // Base58 encoded SecretAggStepOne from step one
    pub recent_block_hash: String,   // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggWithdrawStakeStepTwoResponse {
    pub partial_signature: String, // Base58 encoded PartialSignature
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateStakeSignaturesRequest {
    pub net: Network,
    pub stake_amount: u64, // Amount to stake in lamports
    pub seed: String,      // Seed for stake account
    pub validator_vote_accont: String,
    pub keys: Vec<String>,         // List of pubkeys
    pub signatures: Vec<String>,   // Base58 encoded PartialSignatures
    pub recent_block_hash: String, // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggregateStakeSignaturesResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateDeactivateStakeSignaturesRequest {
    pub net: Network,
    pub stake_account: String,     // Stake account pubkey
    pub keys: Vec<String>,         // List of pubkeys
    pub signatures: Vec<String>,   // Base58 encoded PartialSignatures
    pub recent_block_hash: String, // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggregateDeactivateStakeSignaturesResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateWithdrawStakeSignaturesRequest {
    pub net: Network,
    pub stake_account: String,     // Stake account pubkey
    pub destination: String,       // Destination pubkey
    pub amount: u64,               // Amount to withdraw in lamports
    pub keys: Vec<String>,         // List of pubkeys
    pub signatures: Vec<String>,   // Base58 encoded PartialSignatures
    pub recent_block_hash: String, // Base58 encoded recent blockhash
}

#[derive(Debug, Serialize)]
pub struct AggregateWithdrawStakeSignaturesResponse {
    pub transaction_id: String,
}
