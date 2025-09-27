// # Medium Findings - Recommended Fixes

// ## FM-001: Confusing Function Logic
// CLEAR: Separate functions with clear purposes
//Now this is per recommendation
pub fn get_player_kills(...) -> Result<u16> {
    // Find player and return only kills
    if let Some(team_a_index) = self.team_a.players.iter().position(|p| *p == player_pubkey) {
        Ok(self.team_a.player_kills[team_a_index])
    } else if let Some(team_b_index) = self.team_b.players.iter().position(|p| *p == player_pubkey) {
        Ok(self.team_b.player_kills[team_b_index])
    } else {
        Err(error!(WagerError::PlayerNotFound))
    }
}

pub fn get_player_spawns(&self, player_pubkey: Pubkey) -> Result<u16> {
    // Return spawns remaining for player - similar logic but for spawns only
    // Implementation follows same pattern as get_player_kills
}

// For distribution, use only meaningful metrics:
let player_kills = game_session.get_player_kills(player)?;
let earnings = player_kills as u64 * reward_per_kill;


// ## FM-002: Fixed Array Size Inefficiency
// OPTION 1: Accept the waste (current approach is OK for simplicity)
// Cost analysis: ~$0.16 extra per 1v1 game at current SOL prices
// May be acceptable trade-off for code simplicity
//However, if optimizing for cost: use enums or Options

// OPTION 2: Use Option for better semantics

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Team {
    pub players: [Option<Pubkey>; 5],    // None for empty slots
    pub total_bet: u64,
    pub player_spawns: [u16; 5],
    pub player_kills: [u16; 5],
}
// Cost: +5 bytes per team, but clearer semantics

// Option 3: Use Enum for fixed team sizes
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum Team {
    OneVsOne {
        players: [Pubkey; 1],
        spawns: [u16; 1], 
        kills: [u16; 1],
        total_bet: u64,
    },
    ThreeVsThree {
        players: [Pubkey; 3],
        spawns: [u16; 3],
        kills: [u16; 3], 
        total_bet: u64,
    },
    FiveVsFive {
        players: [Pubkey; 5],
        spawns: [u16; 5],
        kills: [u16; 5],
        total_bet: u64,
    },
}// this has zero space overhead but more complex code



// ## FM-003: Economic Imbalance in Spawn Pricing
// BETTER: Separate pricing for different services
// This relates back to creating a general config account

#[account]
pub struct GameConfig {
    pub entry_fee: u64,           // Cost to join game
    pub spawn_cost: u64,          // Cost per 10 spawns
    pub spawn_multiplier: u16,    // How many spawns per purchase
}

pub fn pay_to_spawn_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    let config = &ctx.accounts.game_config;
    
    // Use spawn-specific pricing
    let spawn_cost = config.spawn_cost;  // E.g., 25% of entry fee
    
    anchor_spl::token::transfer(/* ... */, spawn_cost)?;
    
    // Add configurable number of spawns
    let spawns_to_add = config.spawn_multiplier;
    game_session.add_spawns_amount(team, player_index, spawns_to_add)?;
    
    Ok(())
}


// ## FM-004: No Bet Amount Validation
// SECURE: Add bet amount validation

#[account]
pub struct GameConfig {
    pub min_bet_amount: u64,      // Minimum meaningful bet
    pub max_bet_amount: u64,      // Maximum allowed bet
    pub admin: Pubkey,            // Who can update config
}

pub fn create_game_session_handler(...) -> Result<()> {
    let config = &ctx.accounts.game_config;
    
    // Validate bet amount is within acceptable range
    require!(
        bet_amount >= config.min_bet_amount,
        WagerError::BetTooLow
    );
    
    require!(
        bet_amount <= config.max_bet_amount,
        WagerError::BetTooHigh
    );
    
    let game_session = &mut ctx.accounts.game_session;
    game_session.session_bet = bet_amount;
    
    // ... rest of function
}


// ## FM-005: Integer Overflow in Kill/Spawn Counters

//Use saturating arithmetic (stays at max and is the recommended way)
pub fn add_kill(/* ... */) -> Result<()> {
    match killer_team {
        0 => self.team_a.player_kills[killer_player_index] = 
            self.team_a.player_kills[killer_player_index].saturating_add(1),
        1 => self.team_b.player_kills[killer_player_index] = 
            self.team_b.player_kills[killer_player_index].saturating_add(1),
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    Ok(())
}


// ## FM-006: Session State Enum Incomplete
// COMPLETE: Add all necessary states

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GameStatus {
    WaitingForPlayers,    // Initial state
    InProgress,           // Game is active
    Completed,            // Game finished normally with winner
    Cancelled,            // Game cancelled before starting
    Refunded,             // Game refunded to all players
    Disputed,             // Game result is disputed
    Abandoned,            // Game abandoned/timed out
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::WaitingForPlayers
    }
}

// Use appropriate states:
pub fn refund_wager_handler(/* ... */) -> Result