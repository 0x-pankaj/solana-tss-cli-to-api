use std::fmt::{Display, Formatter};

use bs58::decode::Error as Bs58Error;
use solana_client::client_error::ClientError;
use solana_sdk::program_error::ProgramError;

use crate::serialization::Error as DeserializationError;

#[derive(Debug)]
pub enum Error {
    WrongNetwork(String),
    BadBase58(Bs58Error),
    WrongKeyPair(ed25519_dalek::SignatureError),
    AirdropFailed(ClientError),
    RecentHashFailed(ClientError),
    ConfirmingTransactionFailed(ClientError),
    BalaceFailed(ClientError),
    SendTransactionFailed(ClientError),
    DeserializationFailed {
        error: DeserializationError,
        field_name: &'static str,
    },
    MismatchMessages,
    InvalidSignature,
    KeyPairIsNotInKeys,
    TransactionCreationFailed(String),
    SplTokenError(spl_token::error::TokenError),
    TokenAccountNotFound,
    TokenMintNotFound,
    ProgramError(ProgramError),

    StakeAccountCreationFailed(String),
    InvalidStakeAccountSeed(String),
    StakeDelegationFailed(String),
    DeactivationFailed(String),
    WithdrawalFailed(String),
    InvalidPublicKey(String),
    InsufficientBalance(String),
    BalanceCheckFailed(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongNetwork(net) => write!(
                f,
                "Unrecognized network: {}, please select Mainnet/Testnet/Devnet",
                net
            ),
            Self::BadBase58(e) => write!(f, "Based58 Error: {}", e),
            Self::WrongKeyPair(e) => write!(f, "Failed deserializing keypair: {}", e),
            Self::AirdropFailed(e) => write!(f, "Failed asking for an airdrop: {}", e),
            Self::RecentHashFailed(e) => write!(f, "Failed recieving the latest hash: {}", e),
            Self::ConfirmingTransactionFailed(e) => {
                write!(f, "Failed confirming transaction: {}", e)
            }
            Self::BalaceFailed(e) => write!(f, "Failed checking balance: {}", e),
            Self::SendTransactionFailed(e) => write!(f, "Failed sending transaction: {}", e),
            Self::DeserializationFailed { error, field_name } => {
                write!(f, "Failed deserializing {}: {}", field_name, error)
            }
            Self::MismatchMessages => write!(
                f,
                "There is a mismatch between first_messages and second_messages"
            ),
            Self::InvalidSignature => {
                write!(f, "The resulting signature doesn't match the transaction")
            }
            Self::KeyPairIsNotInKeys => {
                write!(f, "The provided keypair is not in the list of pubkeys")
            }
            Self::TransactionCreationFailed(msg) => {
                write!(f, "Transaction creation failed: {}", msg)
            }
            Self::SplTokenError(e) => write!(f, "SPL Token error: {}", e),
            Self::TokenAccountNotFound => write!(f, "Token account not found"),
            Self::TokenMintNotFound => write!(f, "Token mint not found"),
            Self::ProgramError(e) => write!(f, "Program error: {}", e),

            Self::StakeAccountCreationFailed(e) => {
                write!(f, "Failed to create stake account: {}", e)
            }
            Self::InvalidStakeAccountSeed(e) => write!(f, "Invalid stake account seed: {}", e),
            Self::StakeDelegationFailed(e) => write!(f, "Failed to delegate stake: {}", e),
            Self::DeactivationFailed(e) => write!(f, "Failed to deactivate stake: {}", e),
            Self::WithdrawalFailed(e) => write!(f, "Failed to withdraw stake: {}", e),
            Self::InvalidPublicKey(e) => write!(f, "invalid public key: {}", e),
            Self::InsufficientBalance(e) => write!(f, "insufficient balance: {}", e),
            Self::BalanceCheckFailed(e) => write!(f, " balance check fail: {}", e),
        }
    }
}

impl From<Bs58Error> for Error {
    fn from(e: Bs58Error) -> Self {
        Self::BadBase58(e)
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        Self::WrongKeyPair(e)
    }
}

impl From<spl_token::error::TokenError> for Error {
    fn from(e: spl_token::error::TokenError) -> Self {
        Self::SplTokenError(e)
    }
}

impl From<ProgramError> for Error {
    fn from(e: ProgramError) -> Self {
        Self::ProgramError(e)
    }
}

impl std::error::Error for Error {}
