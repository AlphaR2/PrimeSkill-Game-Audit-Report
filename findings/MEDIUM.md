# Medium Severity Findings

## **FM-001: Confusing Function Logic**

**Severity:** Medium  
**Location:** `get_kills_and_spawns()`

### Vulnerable Code
```rust
// CONFUSING LOGIC: Function name suggests difference, but returns sum
pub fn get_kills_and_spawns(...) -> Result<u16> {
    // Function name implies: "get kills AND spawns" (separate values)
    // Comment suggests: "kill and death difference" (kills - deaths)
    // Actually returns: kills + spawns (addition, not difference)
    
    let team_a_index = self.team_a.players.iter().position(|p| *p == player_pubkey);
    let team_b_index = self.team_b.players.iter().position(|p| *p == player_pubkey);
    
    if let Some(team_a_index) = team_a_index {
        Ok(self.team_a.player_kills[team_a_index] as u16  // Redundant cast
            + self.team_a.player_spawns[team_a_index] as u16)  // What does kills + spawns mean?
    } else if let Some(team_b_index) = team_b_index {
        Ok(self.team_b.player_kills[team_b_index] as u16
            + self.team_b.player_spawns[team_b_index] as u16)
    } else {
        return Err(error!(WagerError::PlayerNotFound));
    }
}

// Used in distribution logic:
let kills_and_spawns = game_session.get_kills_and_spawns(player)?;
let earnings = kills_and_spawns as u64 * game_session.session_bet / 10;
//              This makes no business sense
```

### Problems
```rust
// What does this metric represent?
// Player A: 10 kills, 5 spawns remaining = 15 points
// Player B: 2 kills, 20 spawns remaining = 22 points
// Player B gets higher reward despite worse performance!
```

### Impact
- **Incorrect Rewards:** Players with more spawns get higher payouts
- **Developer Confusion:** Function purpose unclear
- **Economic Imbalance:** Spawn count affects earnings inappropriately

---

## **FM-002: Fixed Array Size Inefficiency**

**Severity:** Medium  
**Location:** Team struct

### Inefficient Code
```rust
// WASTEFUL: Always allocates for 5 players regardless of game mode
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Team {
    pub players: [Pubkey; 5],    // 32 * 5 = 160 bytes
    pub total_bet: u64,          // 8 bytes
    pub player_spawns: [u16; 5], // 2 * 5 = 10 bytes  
    pub player_kills: [u16; 5],  // 2 * 5 = 10 bytes
}
// Total: 188 bytes per team

// Memory waste analysis:
impl GameMode {
    pub fn players_per_team(&self) -> usize {
        match self {
            Self::WinnerTakesAllOneVsOne => 1,        // Uses 1/5 slots = 80% waste
            Self::WinnerTakesAllThreeVsThree => 3,    // Uses 3/5 slots = 40% waste
            Self::WinnerTakesAllFiveVsFive => 5,      // Uses 5/5 slots = 0% waste
            // ... same for PayToSpawn variants
        }
    }
}
```

### Waste Calculation
```rust
// 1v1 Game:
// Used: 1 player × (32 + 2 + 2) = 36 bytes + 8 bytes total_bet = 44 bytes
// Allocated: 188 bytes
// Wasted: 144 bytes per team × 2 teams = 288 bytes per game

// 3v3 Game:
// Used: 3 players × (32 + 2 + 2) = 108 bytes + 8 bytes = 116 bytes  
// Allocated: 188 bytes
// Wasted: 72 bytes per team × 2 teams = 144 bytes per game

// At scale:
// 1000 daily 1v1 games = 288 KB wasted daily = ~8.6 MB monthly
```

### Impact
- **Cost Inefficiency:** Higher rent costs for smaller games
- **Resource Waste:** Unnecessary blockchain storage usage
- **Scalability Issues:** Inefficient at scale

---

## **FM-003: Economic Imbalance in Spawn Pricing**

**Severity:** Medium  
**Location:** `pay_to_spawn_handler()`

### Problematic Code
```rust
// SAME PRICE for different value propositions
pub fn pay_to_spawn_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    let session_bet = game_session.session_bet;  // Same as entry fee

    // Transfer FULL session_bet for just 10 spawns
    anchor_spl::token::transfer(/* ... */, session_bet)?;
    //                                     Same price as joining game!
    
    game_session.add_spawns(team, player_index)?;  // +10 spawns
    Ok(())
}

// Compare with join_user_handler:
pub fn join_user_handler(/* ... */) -> Result<()> {
    // Pay session_bet to join game
    anchor_spl::token::transfer(/* ... */, session_bet)?;
    
    // Get initial spawns (10) + game entry
    selected_team.player_spawns[empty_index] = 10;
    Ok(())
}
```

### Economic Problems
```rust
// Scenario Analysis:
// Entry: Pay 100 tokens → Join game + get 10 spawns
// Additional: Pay 100 tokens → Get 10 spawns only

// Problem 1: Same price, different value
// Entry provides: Game access + 10 spawns
// Additional provides: 10 spawns only
// Both cost the same!

// Problem 2: No pricing discovery
// What if spawns should be cheaper/expensive based on game state?
// No dynamic pricing or market mechanisms

// Problem 3: Economic exploit potential
// In some scenarios, it might be better to leave and rejoin
// rather than buy additional spawns
```

### Impact
- **Pricing Inconsistency:** Same cost for different value propositions
- **Economic Inefficiency:** No market-based pricing
- **Strategy Distortion:** Players might game the pricing system

---

## **FM-004: No Bet Amount Validation**

**Severity:** Medium  
**Location:** `create_game_session_handler()`

### Vulnerable Code
```rust
// NO VALIDATION: Accepts any bet amount
pub fn create_game_session_handler(
    ...
    bet_amount: u64,  // Could be 0 or u64::MAX
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Direct assignment without validation
    game_session.session_bet = bet_amount;  // No min/max checks
    
    // ... rest of function
}
```

### Problems
```rust
// Problem scenarios:
// 1. Zero bet games
create_game_session_handler(ctx, "free_game", 0, game_mode)?;
// Result: Free games that cost nothing to join

// 2. Extreme bet amounts  
create_game_session_handler(ctx, "whale_game", u64::MAX, game_mode)?;
// Result: Games requiring impossible bet amounts

// 3. Dust amounts
create_game_session_handler(ctx, "dust_game", 1, game_mode)?;
// Result: Games with meaningless stakes

// 4. No economic sense
// Small bet: 1 lamport game, gas costs more than prize
// Large bet: Bets larger than total token supply
```

### Impact
- **Economic Nonsense:** Games with invalid economic parameters
- **Platform Exploitation:** Free games bypass monetization
- **User Experience:** Confusing or impossible bet requirements

---

## **FM-005: Integer Overflow in Kill/Spawn Counters**

**Severity:** Medium  
**Location:** Kill and spawn tracking

### Vulnerable Code
```rust
// NO OVERFLOW PROTECTION: Counters can wrap around
pub fn add_kill(/* ... */) -> Result<()> {
    // ... validation ...
    
    match killer_team {
        0 => self.team_a.player_kills[killer_player_index] += 1,  // Can overflow
        1 => self.team_b.player_kills[killer_player_index] += 1,  // Same issue
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    
    // After 65,535 kills, counter wraps to 0!,although very slim chance, it is poor code 
    Ok(())
}

pub fn add_spawns(...) -> Result<()> {
    match team {
        0 => self.team_a.player_spawns[player_index] += 10u16,  // Can overflow
        1 => self.team_b.player_spawns[player_index] += 10u16,  // After 6,553 purchases
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    Ok(())
}
```

### Overflow Scenarios
```rust
// Kill counter overflow:
// Player gets 65,535 kills (unrealistic but possible in long games)
// Next kill: 65,535 + 1 = 0 (wraps around)
// Player's kill count resets to zero!

// Spawn counter overflow:
// Player buys spawns 6,553 times (10 spawns each)
// Total: 65,530 spawns
// Next purchase: 65,530 + 10 = 65,540 → wraps to ~4
// Player loses almost all spawns!
```

### Impact
- **Statistics Corruption:** Game statistics become unreliable
- **Economic Loss:** Players lose spawn purchases due to overflow
- **Competitive Integrity:** Kill counts reset unexpectedly

---

## **FM-006: Session State Enum Incomplete**

**Severity:** Medium  
**Location:** GameStatus enum

### Incomplete Code
```rust
// INCOMPLETE: Missing important game states
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GameStatus {
    WaitingForPlayers, // Waiting for players to join
    InProgress,        // Game is active with all players joined
    Completed,         // Game has finished and rewards distributed
    // Missing: Cancelled state
    // Missing: Refunded state  
    // Missing: Disputed state
    // Missing: Abandoned state
}
```

### Problems
```rust
// Current state transitions are limited:
// WaitingForPlayers → InProgress → Completed

// But what about:
// 1. Games that are cancelled before starting?
// 2. Games that are refunded due to issues?
// 3. Games that are disputed?
// 4. Games abandoned by players?

// Current workaround uses Completed for everything:
pub fn refund_wager_handler(/* ... */) -> Result<()> {
    // ... refund logic ...
    game_session.status = GameStatus::Completed;  // Misleading state
    Ok(())
}
```

### Impact
- **State Confusion:** Same state used for different outcomes
- **Audit Trail Loss:** Cannot distinguish between completion types
- **Logic Complexity:** Additional checks needed to determine actual state

---

## **FM-007: Winner Validation Logic Error**

**Severity:** Medium   
**Location:** Distribution functions

### Redundant Code
```rust
// REDUNDANT: Unnecessary take() on already-sliced array
pub fn distribute_all_winnings_handler(/* ... */) -> Result<()> {
    let players_per_team = game_session.game_mode.players_per_team();

    // Get winning team (already sliced to correct size)
    let winning_players = if winning_team == 0 {
        &game_session.team_a.players[0..players_per_team]  // Already correct size
    } else {
        &game_session.team_b.players[0..players_per_team]  // Already correct size
    };

    for i in 0..players_per_team {
        let winner_pubkey = winner.key();
        
        // REDUNDANT: take() on already-sized slice
        require!(
            winning_players
                .iter()
                .take(players_per_team)  // Unnecessary - slice is already this size
                .any(|&p| p == winner_pubkey),
            WagerError::InvalidWinner
        );
    }
}
```

### Problems
```rust
// The redundancy:
let slice = &array[0..3];        // Slice to 3 elements
slice.iter().take(3)             // Take 3 elements from 3-element slice
// This is redundant - slice is already exactly 3 elements

// Should be:
let slice = &array[0..3];
slice.iter()                     // No take() needed
```

### Impact
- **Code Inefficiency:** Unnecessary operations
- **Developer Confusion:** Redundant logic suggests unclear intent
- **Maintenance Issues:** Extra code to maintain

---

## **FM-008: Fixed Amount Distribution Error**

**Severity:** Medium   
**Location:** Winner-takes-all distribution

### Problematic Code
```rust
// FIXED AMOUNT: Ignores actual vault balance
pub fn distribute_all_winnings_handler(/* ... */) -> Result<()> {
    for i in 0..players_per_team {
        // Calculate fixed amount based on original bet
        let winning_amount = game_session.session_bet * 2;  // Fixed calculation
        
        // Transfer fixed amount regardless of actual vault balance
        anchor_spl::token::transfer(/* ... */, winning_amount)?;
    }
}
```

### Problems
```rust
// Scenario 1: Pay-to-spawn game with extra funds
// Original vault: 200 tokens (2 players × 100)
// Pay-to-spawn additions: 500 tokens
// Total vault: 700 tokens
// Distribution: 2 × 100 = 200 tokens (fixed)
// Leftover: 500 tokens stranded in vault!

// Scenario 2: Partial game
// Some players joined but didn't deposit full amount
// Vault has less than expected
// Distribution tries to pay more than available
// Transaction fails or partial payments

// Scenario 3: Fee deductions
// Platform takes 2% fee = 4 tokens from 200
// Vault has 196 tokens
// Distribution tries to pay 200 tokens
// Fails due to insufficient funds
```

### Impact
- **Stranded Funds:** Tokens left permanently in vaults
- **Distribution Failures:** Attempts to pay more than available
- **Economic Inefficiency:** Players don't receive full winnings

---

## **FM-009: Race Condition in Team Filling**

**Severity:** Medium  
**Location:** `join_user_handler()`

### Vulnerable Code
```rust
// NO ATOMIC OPERATIONS: Multiple players can join simultaneously
pub fn join_user_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Check for empty slot
    let empty_index = game_session.get_player_empty_slot(team)?;  // Time A
    
    // ... token transfer and other operations ...
    
    // Add player to slot
    selected_team.players[empty_index] = player.key();  // Time B
    
    // Check if game is full
    if game_session.check_all_filled()? {
        game_session.status = GameStatus::InProgress;
    }
}
```

### Race Condition Scenario
```rust
// Timeline:
// Time 0: Game has 1 empty slot in each team
// Time 1: Player A calls join_user_handler(team=0)
// Time 2: Player B calls join_user_handler(team=0) 
// Time 3: Both get empty_index = 1 (same slot!)
// Time 4: Player A executes: team.players[1] = player_a
// Time 5: Player B executes: team.players[1] = player_b
// Result: Player A gets overwritten, only Player B is in team

// Another scenario:
// Time 1: Player A calls join, team now has 2/2 players
// Time 2: Player B calls join, also sees 2/2 players  
// Time 3: Both set status to InProgress
// Time 4: Game starts with inconsistent state
```

### Impact
- **Player Overwrites:** Later players overwrite earlier ones
- **Inconsistent States:** Game status conflicts
- **Lost Deposits:** Overwritten players lose their deposits

---

## **FM-010: Missing Refund State Tracking**

**Severity:** Medium  
**Location:** Refund system

### Current Issue
```rust
// NO REFUND TRACKING: Cannot tell who was refunded
pub fn refund_wager_handler(/* ... */) -> Result<()> {
    let players = game_session.get_all_players();

    for player in players {
        if player == Pubkey::default() {
            continue;
        }

        let refund = game_session.session_bet;
        
        // No tracking: Who has been refunded?
        // No state: Is refund complete or partial?
        // No verification: Was refund successful?
        
        anchor_spl::token::transfer(/* ... */, refund)?;
    }

    game_session.status = GameStatus::Completed;  // No refund-specific state
    Ok(())
}
```

### Problems
```rust
// Issues:
// 1. No way to know which players received refunds
// 2. If function fails halfway, cannot resume
// 3. No audit trail of refund process
// 4. Cannot prevent double refunds
// 5. No verification that all players were refunded
```

### Impact
- **Incomplete Refunds:** Process can fail without recovery
- **No Audit Trail:** Cannot verify refund completion
- **Double Spend Risk:** No protection against multiple refunds

---

## **FM-011: Data Type Size Optimization**

**Severity:** Medium  
**Location:** player_spawns and player_kills arrays

### Current Inefficient Types
```rust
// OVERSIZED: u16 for values that rarely exceed 255
pub struct Team {
    pub players: [Pubkey; 5],
    pub total_bet: u64,
    pub player_spawns: [u16; 5], // 2 bytes × 5 = 10 bytes
    pub player_kills: [u16; 5],  // 2 bytes × 5 = 10 bytes
}
// Total for spawns + kills: 20 bytes per team
```

### Analysis
```rust
// Realistic value ranges:
// player_spawns: Typically 0-50 spawns per player (max needed: u8)
// player_kills: Typically 0-100 kills per player (max needed: u8)

// Current capacity with u16:
// player_spawns: 0 to 65,535 (massive overkill)
// player_kills: 0 to 65,535 (massive overkill)

// Memory savings with u8:
// player_spawns: [u8; 5] = 1 byte × 5 = 5 bytes (-5 bytes)
// player_kills: [u8; 5] = 1 byte × 5 = 5 bytes (-5 bytes)
// Total savings: 10 bytes per team = 20 bytes per game
```

### Trade-off Analysis
```rust
// Benefits of u8:
// + 20 bytes saved per game
// + Lower serialization costs
// + More cache-friendly data structures

// Risks of u8:
// - player_spawns limited to 255 max
// - player_kills limited to 255 max
// - Need overflow protection

// Is 255 enough?
// Spawns: 255 spawns is ~25 spawn purchases (10 each) - reasonable limit
// Kills: 255 kills per player in one game - very high, probably sufficient
```

### Impact
- **Memory Efficiency:** 20 bytes saved per game
- **Cost Reduction:** Lower rent and transaction costs
- **Performance:** Faster serialization/deserialization

---

## **FM-012: Redundant Type Casting**

**Severity:** Medium  
**Location:** Various calculations

### Redundant Code Examples
```rust
// REDUNDANT: Casting u16 to u16
pub fn get_kills_and_spawns(&self, player_pubkey: Pubkey) -> Result<u16> {
    if let Some(team_a_index) = team_a_index {
        Ok(self.team_a.player_kills[team_a_index] as u16      // Already u16
            + self.team_a.player_spawns[team_a_index] as u16) // Already u16
    } else if let Some(team_b_index) = team_b_index {
        Ok(self.team_b.player_kills[team_b_index] as u16      // Redundant
            + self.team_b.player_spawns[team_b_index] as u16) // Redundant
    } else {
        return Err(error!(WagerError::PlayerNotFound));
    }
}

// More examples throughout codebase:
let earnings = kills_and_spawns as u64 * game_session.session_bet / 10;
//              If kills_and_spawns is already proper type

// Pattern appears in multiple locations:
// - Distribution calculations
// - Statistical computations  
// - Mathematical operations
```

### Problems
```rust
// Issues with redundant casting:
// 1. Code confusion - suggests type uncertainty
// 2. Maintenance overhead - extra code to review
// 3. Performance impact - unnecessary operations
// 4. Reader confusion - why is cast needed?

// struct definitions:
pub player_spawns: [u16; 5],  // These are u16
pub player_kills: [u16; 5],   // These are u16

// So casting to u16 is unnecessary:
self.team_a.player_kills[index] as u16  // u16 → u16 (no-op)
```

### Impact
- **Code Clarity:** Unnecessary casts suggest confusion about types
- **Maintenance:** Extra code to review and maintain
- **Performance:** Minor overhead from unnecessary operations

---

## **FM-013: Missing Config Account**

**Severity:** Medium  
**Location:** System-wide

### Current Hardcoded Values
```rust
// HARDCODED: Values scattered throughout code
pub fn add_spawns(&mut self, team: u8, player_index: usize) -> Result<()> {
    match team {
        0 => self.team_a.player_spawns[player_index] += 10u16,  // Hardcoded 10
        1 => self.team_b.player_spawns[player_index] += 10u16,  // Hardcoded 10
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
}

// More hardcoded values:
selected_team.player_spawns[empty_index] = 10;  // Initial spawns
let earnings = kills_and_spawns as u64 * session_bet / 10;  // Hardcoded divisor
// No minimum/maximum bet validation
// No configurable game parameters
// No admin controls for game mechanics
```

### Problems
```rust
// Issues with hardcoded values:
// 1. Cannot adjust game balance without code changes
// 2. No A/B testing of different parameters
// 3. Difficult to respond to economic issues
// 4. No admin controls for game management
// 5. Parameters scattered across multiple files

// Example scenarios where config would help:
// - Spawn cost becomes too cheap/expensive
// - Initial spawn count needs adjustment
// - Reward multipliers need balancing
// - Minimum bet amounts need updates
```

### Impact
- **Inflexibility:** Cannot adjust game parameters without redeployment
- **Maintenance Difficulty:** Parameters scattered across codebase
- **Economic Risk:** Cannot respond quickly to balance issues
- **Scalability:** No way to test different configurations

---
