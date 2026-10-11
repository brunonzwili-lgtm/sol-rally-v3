use anchor_lang::prelude::*;

declare_id!("GtzDo8g279ocmbeUUSLUEKrYBBUsGWWYYWkUjHoR9TRe");

#[program]
pub mod sol_rally_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
