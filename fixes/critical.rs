// Here we have all the critical fixes that need to be applied to the game

//FC-001: Incorrect calculation of account size due to misunderstanding of data types


// CORRECT: Use proper data type sizes
space = 8 + 4 + 10 + 32 + 8 + 1 + (2 * (32 * 5 + 2 * 5 + 2 * 5 + 8)) + 1 + 8 + 3
// Or better: Use Anchor's InitSpace macro
#[account]
#[derive(InitSpace)]
pub struct GameSession { ... }

//FC-002:Integer Underflow in Spawn System

// SECURE: Add underflow protection
match victim_team {
    0 => {
        if self.team_a.player_spawns[victim_player_index] > 0 {
            self.team_a.player_spawns[victim_player_index] -= 1;
        } else {
            return Err(error!(WagerError::NoSpawnsRemaining)); // OR ANY CUSTOM ERROR 
        }
    },
    1 => {
        if self.team_b.player_spawns[victim_player_index] > 0 {
            self.team_b.player_spawns[victim_player_index] -= 1;
        } else {
            return Err(error!(WagerError::NoSpawnsRemaining));
        }
    },
    _ => return Err(error!(WagerError::InvalidTeam)),
}

//FC-003:Missing Session ID Length Validation

// SECURE: Add length validation
pub fn create_game_session_handler(
  ...
) -> Result<()> {
    // Validate session_id length
    require!(session_id.len() <= 10, WagerError::SessionIdTooLong);
    
    let game_session = &mut ctx.accounts.game_session;
    game_session.session_id = session_id;
    // ... rest of function
}

//FC-004:Dangerous AccountInfo Usage

// ADD A VAULTSTATE ACCOUNT TO TRACK VAULT INFO THEN THE VAULT ITSELF TO BE A PDA

// SECURE: Create proper vault state structure
#[account]
pub struct VaultState {
    pub game_session: Pubkey,     // Which game this vault belongs to
    pub expected_total: u64,      // Total that should be escrowed
    pub current_balance: u64,     // Track actual deposits
    pub players_deposited: u8,    // How many players have deposited
    pub is_active: bool,          // Vault status
    pub bump: u8,                 // PDA bump
}

// Use proper typed account
#[account(
    init,
    payer = game_server,
    space = 8 + VaultState::INIT_SPACE,
    seeds = [b"vault", session_id.as_bytes()],
    bump
)]
pub vault_state: Account<'info, VaultState>,  // Type-safe with state tracking


//FC-005:Vault Balance Reconciliation Missing

// SECURE: Verify complete vault drainage
pub fn distribute_all_winnings_handler(/* ... */) -> Result<()> {
    let vault_balance = ctx.accounts.vault_token_account.amount;
    let total_players = players_per_team * 2;
    let amount_per_winner = vault_balance / players_per_team as u64;
    
    // Distribute actual vault balance, not fixed amounts
    for i in 0..players_per_team {
        anchor_spl::token::transfer(/* ... */, amount_per_winner)?;
    }
    
    // Verify vault is completely empty
    let remaining_balance = ctx.accounts.vault_token_account.amount;
    require!(remaining_balance == 0, WagerError::VaultNotEmpty);
    
    game_session.status = GameStatus::Completed;
    Ok(())
}

//FC-006:Missing Duplicate Player Check


// SECURE: Check for duplicate players
pub fn join_user_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    let player = ctx.accounts.user.key();
    
    // Check if player already exists in either team
    let all_players = game_session.get_all_players();
    require!(
        !all_players.contains(&player),
        WagerError::PlayerAlreadyInGame
    );
    
    let empty_index = game_session.get_player_empty_slot(team)?;
    // ... rest of function safely
}

//FC-007:Code Struct Space Implementation

// SECURE: Use Anchor's automatic space calculation
//remember to calculate space for the enums differently

#[account]
#[derive(InitSpace)]
pub struct GameSession {
    #[max_len(10)]
    pub session_id: String,
    pub authority: Pubkey,
    pub session_bet: u64,
    pub game_mode: GameMode,
    pub team_a: Team,
    pub team_b: Team,
    pub status: GameStatus,
    pub created_at: i64,
    pub bump: u8,
    pub vault_bump: u8,
    pub vault_token_bump: u8,
}
// add InitSpace here
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, InitSpace)]
pub struct Team {
    pub players: [Pubkey; 5],
    pub total_bet: u64,
    pub player_spawns: [u16; 5],
    pub player_kills: [u16; 5],
}

// for enums

/// Status of a game session
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GameStatus {
    WaitingForPlayers, // Waiting for players to join
    InProgress,        // Game is active with all players joined
    Completed,         // Game has finished and rewards distributed
    //add a state for refund. 
}
impl InitSpace for GameStatus {
    const INIT_SPACE: usize = 1;
}


// Anchor automatically calculates correct space
#[account(
    init,
    payer = game_server,
    space = 8 + GameSession::INIT_SPACE,  // Reliable calculation
    seeds = [b"game_session", session_id.as_bytes()],
    bump
)]
pub game_session: Account<'info, GameSession>,






