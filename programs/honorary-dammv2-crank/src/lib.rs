use anchor_lang::prelude::*;

declare_id!("ddcEKSibupo9XMaeHH66rVkpqCpWybXtAZWaBbMbF3h");

#[program]
pub mod honorary_dammv2_crank {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
