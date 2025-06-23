use crate::Error;
use solana_sdk::{
    instruction::Instruction, message::Message, pubkey::Pubkey, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::instruction as token_instruction;
use solana_client::rpc_client::RpcClient;


pub fn create_spl_token_transaction(
    amount: u64,
    from: &Pubkey,
    to: &Pubkey,
    token_mint: &Pubkey,
    payer: &Pubkey,
    memo: Option<String>,
    decimals: u8,
) -> Result<Transaction, Error> {
    let mut instructions = Vec::new();

    // Get associated token addresses
    let from_ata = get_associated_token_address(from, token_mint);
    let to_ata = get_associated_token_address(to, token_mint);

    let rpc_client = RpcClient::new("https://api.testnet.solana.com".to_string());

    if rpc_client.get_account(&from_ata).is_err() {

    // For now,  always try to create it (instruction will fail if it already exists)
    let create_ata_instruction = create_associated_token_account(
        payer, // fee payer
        to,    // wallet owner
        token_mint,
        &spl_token::id(),
    );
    instructions.push(create_ata_instruction);
    }


    if rpc_client.get_account(&to_ata).is_err() {
    let create_to_ata_instruction = create_associated_token_account(
            payer,     // fee payer
            to,        // wallet owner
            token_mint,
            &spl_token::id(),
        );
        instructions.push(create_to_ata_instruction);

    }

    // Create the token transfer instruction
    let transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &from_ata, // source token account
        &to_ata,   // destination token account
        from,      // source account owner
        &[],       // signer pubkeys (empty for single signer)
        amount,
    )?;
    instructions.push(transfer_instruction);
    

    //  memo instruction if provided
    if let Some(memo_text) = memo {
        let memo_instruction = Instruction {
            program_id: spl_memo::id(),
            accounts: Vec::new(),
            data: memo_text.into_bytes(),
        };
        instructions.push(memo_instruction);
    }

    let message = Message::new(&instructions, Some(payer));
    Ok(Transaction::new_unsigned(message))
}

pub fn get_token_amount_with_decimals(amount: f64, decimals: u8) -> u64 {
    (amount * 10_f64.powi(decimals as i32)) as u64
}
