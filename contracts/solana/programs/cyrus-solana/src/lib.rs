use anchor_lang::prelude::*;

declare_id!("HTViMNg8vW2SV7cesRAoFMcfJj2wH9NAqsb6NeraMBsk");

#[program]
pub mod cyrus_solana {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
