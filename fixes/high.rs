// FH-001: Array Bounds Vulnerability

// SECURE: Add bounds checking
pub fn add_spawns(&mut self, team: u8, player_index: usize) -> Result<()> {
    require!(player_index < 5, WagerError::InvalidPlayerIndex); //fix here
    
    match team {
        0 => self.team_a.player_spawns[player_index] += 10u16,
        1 => self.team_b.player_spawns[player_index] += 10u16,
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    Ok(())
}


// FH-002: No Spawn Limit Validation + Economic Model Flaw

// SECURE: Add spawn limits and fix economic model

// Not complete code, just the relevant parts and sample

pub fn pay_to_spawn_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    let player_index = game_session.get_player_index(team, ctx.accounts.user.key())?;
    
    // Check current spawn count
    let current_spawns = match team {
        0 => game_session.team_a.player_spawns[player_index],
        1 => game_session.team_b.player_spawns[player_index],
        _ => return Err(error!(WagerError::InvalidTeam)),
    };
    
    // Enforce maximum spawn limit
    const MAX_SPAWNS_PER_PLAYER: u16 = 20; // Example limit
    require!(
        current_spawns + 10 <= MAX_SPAWNS_PER_PLAYER,
        WagerError::TooManySpawns
    );
    
    // Use different pricing for spawns vs entry
    let spawn_cost = game_session.session_bet / 4;  // 25% of entry fee
    anchor_spl::token::transfer(/* ... */, spawn_cost)?;
    
    game_session.add_spawns(team, player_index)?;
    Ok(())
}

// Fix reward calculation - only reward kills, not spawns
pub fn distribute_pay_spawn_earnings(/* ... */) -> Result<()> {
    for player in players {
        let player_kills = game_session.get_player_kills(player)?;  // Only kills
        let earnings = player_kills as u64 * game_session.session_bet / 10;
        
        if earnings > 0 {
            anchor_spl::token::transfer(/* ... */, earnings)?;
        }
    }
    Ok(())
}

// FH-003: Team Index Validation Missing


// SECURE: Use enum for type safety
#[derive(Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum TeamSide {
    A = 0,
    B = 1,
}

pub fn join_user_handler(ctx: Context<JoinUser>, _session_id: String, team_side: TeamSide) -> Result<()> {
    // No validation needed - enum guarantees valid values
    let empty_index = game_session.get_player_empty_slot(team_side)?;
    // ... rest of function
}

pub fn get_player_empty_slot(&self, team_side: TeamSide) -> Result<usize> {
    match team_side {
        TeamSide::A => self.team_a.get_empty_slot(player_count),
        TeamSide::B => self.team_b.get_empty_slot(player_count),
    }
}

// FH-004: No Game State Validation

// SECURE: Add proper state validation
pub fn join_user_handler(ctx: Context<JoinUser>, _session_id: String, team: u8) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Validate game state
    require!(
        game_session.status == GameStatus::WaitingForPlayers,
        WagerError::GameNotAcceptingPlayers
    );
    
    // ... rest of function
}

pub fn refund_wager_handler(/* ... */) -> Result<()> {
    let game_session = &ctx.accounts.game_session;
    
    // Only allow refunds for waiting games
    require!(
        game_session.status == GameStatus::WaitingForPlayers,
        WagerError::InvalidRefundState
    );
    
    // ... rest of function
    game_session.status = GameStatus::Refunded;  // Proper final state
}


// FH-005: Remaining Accounts Design Flaw

// SECURE: Use stored player data and derive ATAs
pub fn distribute_all_winnings_handler(
 ...
) -> Result<()> {
    let game_session = &ctx.accounts.game_session;
    let players_per_team = game_session.game_mode.players_per_team();
    
    // Get winners from stored data
    let winning_players = if winning_team == 0 {
        &game_session.team_a.players[0..players_per_team]
    } else {
        &game_session.team_b.players[0..players_per_team]
    };
    
    // Use stored data directly - no remaining_accounts needed
    for &winner_pubkey in winning_players {
        if winner_pubkey == Pubkey::default() {
            continue;  // Skip empty slots
        }
        
        // Derive ATA deterministically
        let winner_ata = get_associated_token_address(&winner_pubkey, &TOKEN_ID);
        
        // Create transfer instruction with derived accounts
        // Much simpler and safer than remaining_accounts
    }
}

// FH-006: No Authority Validation

// SECURE: Add consistent authority validation
pub fn record_kill_handler(
 ...
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Validate authority
    require!(
        game_session.authority == ctx.accounts.game_server.key(),
        WagerError::UnauthorizedOperation
    );
    
    game_session.add_kill(killer_team, killer, victim_team, victim)?;
    Ok(())
}

// Add to all sensitive operations
#[derive(Accounts)]
pub struct RecordKill<'info> {
    #[account(
        mut,
        constraint = game_session.authority == game_server.key() @ WagerError::UnauthorizedOperation,
    )]
    pub game_session: Account<'info, GameSession>,
    
    pub game_server: Signer<'info>,
}

// or create a central config account with authority info and use that for easy validation


// FH-007: Double Spend Vulnerability
// SECURE: Add refund state tracking
#[account]
pub struct RefundState {
    pub game_session: Pubkey,
    pub players_refunded: [Pubkey; 10], // Max 10 players
    pub total_refunded: u64,
    pub refund_completed: bool,
}

pub fn refund_wager_handler(
  ...
) -> Result<()> {
    let game_session = &ctx.accounts.game_session;
    let refund_state = &mut ctx.accounts.refund_state;
    
    // Check if refund already completed
    require!(!refund_state.refund_completed, WagerError::RefundAlreadyCompleted);
    
    let players = game_session.get_all_players();

    for player in players {
        if player == Pubkey::default() {
            continue;
        }

        // Check if player already refunded
        require!(
            !refund_state.players_refunded.contains(&player),
            WagerError::PlayerAlreadyRefunded
        );

        let refund = game_session.session_bet;
        
        // Process refund
        anchor_spl::token::transfer(/* ... */, refund)?;
        
        // Track refunded player
        // get the refunded_state and add the player
        // refund_state.players_refunded; 
        refund_state.total_refunded += refund;
    }

    // Mark refund as completed
    refund_state.refund_completed = true;
    game_session.status = GameStatus::Refunded;
    Ok(())
}



// FH-008: Kill Recording Missing Validation

// SECURE: Add comprehensive validation
pub fn record_kill_handler(
 ...
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Validate game state
    require!(
        game_session.status == GameStatus::InProgress,
        WagerError::GameNotInProgress
    );
    
    // Validate killer is in killer_team
    let killer_exists = match killer_team {
        0 => game_session.team_a.players.contains(&killer),
        1 => game_session.team_b.players.contains(&killer),
        _ => return Err(error!(WagerError::InvalidTeam)),
    };
    require!(killer_exists, WagerError::KillerNotInTeam);
    
    // Validate victim is in victim_team
    let victim_exists = match victim_team {
        0 => game_session.team_a.players.contains(&victim),
        1 => game_session.team_b.players.contains(&victim),
        _ => return Err(error!(WagerError::InvalidTeam)),
    };
    require!(victim_exists, WagerError::VictimNotInTeam);
    
    // Validate killer != victim
    require!(killer != victim, WagerError::SelfKillNotAllowed);
    
    // Validate different teams (no friendly fire)
    require!(killer_team != victim_team, WagerError::FriendlyFireNotAllowed);
    
    game_session.add_kill(killer_team, killer, victim_team, victim)?;
    Ok(())
}


// FH-009: Vault Seed Security Weakness

// SECURE: Include game session key for uniqueness
#[account(
    init,
    payer = game_server,
    space = 8 + VaultState::INIT_SPACE,
    seeds = [
        b"vault", 
        game_session.key().as_ref(),    // Include game session PDA
        session_id.as_bytes()           //  Plus session_id for clarity
    ],
    bump
)]
pub vault_state: Account<'info, VaultState>,

// Alternative: Include authority for additional uniqueness
seeds = [
    b"vault",
    game_session.authority.as_ref(),   // Authority pubkey
    session_id.as_bytes()
],

// This ensures each game has unique vault regardless of session_id collisions