use anchor_lang::prelude::*;

declare_id!("HTViMNg8vW2SV7cesRAoFMcfJj2wH9NAqsb6NeraMBsk");

#[program]
pub mod cyrus_solana {
    use super::*;

    pub fn request_settlement(
        ctx: Context<RequestSettlement>,
        amount_usdc: u64,  
        aptos_recipient: String, // Aptos address as string
    ) -> Result<()> {
        let clock = Clock::get()?;
        
        
        let nonce = clock.slot;
        
        let instruction_sysvar = ctx.accounts.instruction_sysvar.to_account_info();
        
        msg!("Cyrus Protocol Settlement Request");
        msg!("Amount: {} micro USDC ({} USDC)", amount_usdc, amount_usdc as f64 / 1_000_000.0);
        msg!("Aptos Recipient: {}", aptos_recipient);
        msg!("Nonce: {}", nonce);
        msg!("Slot: {}", clock.slot);
        msg!("Timestamp: {}", clock.unix_timestamp);
        
        emit!(SettlementRequested {
            source_chain: "solana".to_string(),
            
            aptos_recipient: aptos_recipient.clone(),
            amount: amount_usdc,
            nonce,
            slot: clock.slot,
            timestamp: clock.unix_timestamp as u64,
        });
        
        msg!("SETTLEMENT_EVENT: {{\"aptos_recipient\":\"{}\",\"amount\":{},\"nonce\":{},\"slot\":{},\"timestamp\":{}}}", 
             aptos_recipient, amount_usdc, nonce, clock.slot, clock.unix_timestamp);
        
        Ok(())
    }
    
    pub fn emit_settlement(
        ctx: Context<RequestSettlement>,
        aptos_recipient: String,
        amount_usdc: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let nonce = clock.slot;
        
        msg!("Cyrus Settlement: {} micro USDC → {}", amount_usdc, aptos_recipient);
        
        // Emit the event
        emit!(SettlementRequested {
            source_chain: "solana".to_string(),
            aptos_recipient,
            amount: amount_usdc,
            nonce,
            slot: clock.slot,
            timestamp: clock.unix_timestamp as u64,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RequestSettlement<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// CHECK: This is the instruction sysvar account
    #[account(address = anchor_lang::solana_program::sysvar::instructions::id())]
    pub instruction_sysvar: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[event]
pub struct SettlementRequested {
    pub source_chain: String,
    pub aptos_recipient: String,
    pub amount: u64,
    pub nonce: u64,
    pub slot: u64,
    pub timestamp: u64,
}