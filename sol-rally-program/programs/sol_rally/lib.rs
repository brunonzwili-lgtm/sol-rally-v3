use anchor_lang::prelude::*;

declare_id!("GtzDo8g279ocmbeUUSLUEKrYBBUsGWWYYWkUjHoR9TRe");

#[account]
#[derive(InitSpace)]
pub struct GrandPrix {
    pub authority: Pubkey,
    pub bump: u8,
    pub entry_fee: u64,
    pub total_fees: u64,
    pub total_entried: u64,
}

#[derive(Accounts)]
#[instruction(entry_fee: u64)]
pub struct InitGp<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + GrandPrix::INIT_SPACE, seeds = [b"gp", authority.key().as_ref()], bump)]
    pub gp: Account<'info, GrandPrix>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction()]
pub struct EnterGp<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [b"gp", gp.authority.as_ref()], bump = gp.bump)]
    pub gp: Account<'info, GrandPrix>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveGp<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, close = authority, seeds = [b"gp", gp.authority.as_ref()], bump = gp.bump)]
    pub gp: Account<'info, GrandPrix>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelGp<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, close = authority, seeds = [b"gp", gp.authority.as_ref()], bump = gp.bump)]
    pub gp: Account<'info, GrandPrix>,
}

#[program]
pub mod sol_rally {
    use super::*;

    pub fn init_gp(ctx: Context<InitGp>, entry_fee: u64) -> Result<()> {
        let gp = &mut ctx.accounts.gp;
        gp.authority = ctx.accounts.authority.key();
        gp.bump = *ctx.bumps.get("gp").unwrap();
        gp.entry_fee = entry_fee;
        gp.total_fees = 0;
        gp.total_entried = 0;
        Ok(())
    }

    pub fn enter_gp(ctx: Context<EnterGp>) -> Result<()> {
        let gp = &mut ctx.accounts.gp;

        let player_lamports = ctx.accounts.player.lamports.borrow();
        let gp_lamports = ctx.accounts.gp.lamports.borrow_mut();

        if *player_lamports < gp.entry_fee {
            return Err(GrandPrixError::InsufficientEntryFee.into());
        }

        *player_lamports -= gp.entry_fee;
        *gp_lamports += gp.entry_fee;
        gp.total_fees += gp.entry_fee;
        gp.total_entried += 1;
        Ok(())
    }

    pub fn resolve_gp(ctx: Context<ResolveGp>) -> Result<()> {
        let gp = &mut ctx.accounts.gp;
        let total_fees = gp.total_fees;

        // 50/30/20 split
        let first = total_fees * 50 / 100;
        let second = total_fees * 30 / 100;
        let third = total_fees * 20 / 100;

        // Transfer prizes to authority
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.gp.to_account_info(),
                    to: ctx.accounts.authority.to_account_info(),
                },
            ),
            first,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.gp.to_account_info(),
                    to: ctx.accounts.authority.to_account_info(),
                },
            ),
            second,
        )?;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.gp.to_account_info(),
                    to: ctx.accounts.authority.to_account_info(),
                },
            ),
            third,
        )?;

        let remaining = total_fees - first - second - third;
        if remaining > 0 {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.gp.to_account_info(),
                        to: ctx.accounts.authority.to_account_info(),
                    },
                ),
                remaining,
            )?;
        }

        // gp account closed via close=authority sends remaining lamports to authority
        Ok(())
    }

    pub fn cancel_gp(ctx: Context<CancelGp>) -> Result<()> {
        // gp account closed via close=authority sends lamports to authority
        Ok(())
    }
}

#[error_code]
pub enum GrandPrixError {
    InsufficientEntryFee,
}