# High Severity Findings

## **FH-001: Array Bounds Vulnerability**

**Severity:** High  
**Location:** `add_spawns()` and related functions

### Vulnerable Code
```rust
// NO BOUNDS CHECKING: player_index could be any value
pub fn add_spawns(&mut self, team: u8, player_index: usize) -> Result<()> {
    match team {
        0 => self.team_a.player_spawns[player_index] += 10u16,  // No validation if player_index > 4
        1 => self.team_b.player_spawns[player_index] += 10u16,  // Same issue
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    Ok(())
}

// Problem: Arrays are fixed size [T; 5] but player_index is unchecked
// pub player_spawns: [u16; 5]  // Valid indices: 0, 1, 2, 3, 4
```

### Attack Vector
```rust
// Attacker passes invalid player_index
add_spawns(0, 100)?;  // player_index = 100, but array only has indices 0-4
// Result: Runtime panic - "index out of bounds: the len is 5 but the index is 100"
```

### Impact
- **Program Crash:** Transaction fails with panic
- **DoS Attack:** Easy to trigger by passing large indices
- **System Instability:** Unpredictable behavior

---

## **FH-002: No Spawn Limit Validation + Economic Model Flaw**

**Severity:** High    
**Location:** `pay_to_spawn_handler()` and `distribute_pay_spawn_earnings()`

### Vulnerable Code - No Spawn Limits
```rust
// NO MAXIMUM SPAWN LIMIT: Players can buy unlimited spawns
pub fn pay_to_spawn_handler(ctx: Context<PayToSpawn>, _session_id: String, team: u8) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    ...
    // Transfer payment (always session_bet amount)
    anchor_spl::token::transfer(/* ... */, session_bet)?;

    // Add spawns without any limit check
    game_session.add_spawns(team, player_index)?;  //No max spawn validation
    
    Ok(())
}

pub fn add_spawns(&mut self, team: u8, player_index: usize) -> Result<()> {
    match team {
        0 => self.team_a.player_spawns[player_index] += 10u16,  //Can overflow u16
        1 => self.team_b.player_spawns[player_index] += 10u16,  //No maximum limit
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    Ok(())
}
```

### Vulnerable Code - Economic Model Flaw
```rust
// BROKEN INCENTIVES: Rewards buying spawns instead of skill
pub fn distribute_pay_spawn_earnings(/* ... */) -> Result<()> {
  ...
    for player in players {
        let kills_and_spawns = game_session.get_kills_and_spawns(player)?;
        
         if kills_and_spawns == 0 { // skips default pubkeys but also players with 0 kills and 0 spawns
            continue;
        }

        let earnings = kills_and_spawns as u64 * game_session.session_bet / 10;
        //              ^^^^^^^^^^^^^^^^
        // This adds KILLS + SPAWNS - rewards players for buying more spawns!
    }
    Ok(())
}

pub fn get_kills_and_spawns(&self, player_pubkey: Pubkey) -> Result<u16> {
    // ... find player ...
    Ok(self.team_a.player_kills[team_a_index] + self.team_a.player_spawns[team_a_index])
    //      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //      Skill-based (good)                  Spawn count (bad incentive!)
}
```

### Economic Attack Scenario
```rust
// Attack: Buy spawns to increase payout
// 1. Player joins 1v1 game: pays 100 tokens, gets 10 spawns
// 2. Player buys spawns 10 times: pays 1000 more tokens, now has 110 spawns
// 3. Player gets 5 kills during game
// 4. Payout calculation: (5 kills + 110 spawns) * 100 / 10 = 1,150 tokens
// 5. Player invested 1,100 tokens, earned 1,150 tokens = 50 tokens profit
// 6. Opponent who played skillfully gets much less despite better performance
```

### Impact
- **Game Imbalance:** Rich players can buy advantages
- **Economic Exploitation:** Spawn purchases become profitable
- **Skill Irrelevance:** Performance matters less than spending
- **Game Integrity Loss:** Pay-to-win mechanics destroy fair play


---

## **FH-003: Team Index Validation Missing**

**Severity:** High   
**Location:** Multiple functions using team: u8

### Vulnerable Code
```rust
// UNSAFE: u8 can hold values 0-255, but only 0 and 1 are valid
pub fn join_user_handler(...) -> Result<()> {
    // Basic validation only checks 0 or 1
    require!(team == 0 || team == 1, WagerError::InvalidTeamSelection);
    //       ^^^^^^^^^^^^^^^^^^^ What if team = 2, 13, 100, 255?
    
    let empty_index = game_session.get_player_empty_slot(team)?;
    // ... rest of function
}

pub fn get_player_empty_slot(&self, team: u8) -> Result<usize> {
    match team {
        0 => self.team_a.get_empty_slot(player_count),
        1 => self.team_b.get_empty_slot(player_count),
        _ => Err(error!(WagerError::InvalidTeam)),  // Catches invalid values but after processing
    }
}

// Same pattern in ALL functions:
// - pay_to_spawn_handler(team: u8)
// - add_kill(killer_team: u8, victim_team: u8)
// - add_spawns(team: u8)
```

### Attack Vector
```rust
// System could accidentally pass invalid team values
let malicious_team: u8 = 13;
join_user_handler(ctx, session_id, malicious_team)?;  // Could cause undefined behavior
```

### Impact
- **Type Safety Loss:** No compile-time protection against invalid values
- **Runtime Errors:** Functions fail with unclear error messages
- **Code Fragility:** Easy to introduce bugs when modifying team logic

---

## **FH-004: No Game State Validation**

**Severity:** High  
**Location:** Multiple handlers

### Vulnerable Code
```rust
// NO STATE VALIDATION in join_user_handler
pub fn join_user_handler(ctx: Context<JoinUser>, _session_id: String, team: u8) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // No check: What if game is already completed?
    // No check: What if game is in progress?
    // No check: What if game is cancelled?
    
    let empty_index = game_session.get_player_empty_slot(team)?;
    // ... player joins game that shouldn't accept new players
}

// PARTIAL STATE VALIDATION in pay_to_spawn_handler
pub fn pay_to_spawn_handler(ctx: Context<PayToSpawn>, _session_id: String, team: u8) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;

    // Some validation but incomplete
    require!(
        game_session.status == GameStatus::InProgress && game_session.is_pay_to_spawn(),
        WagerError::InvalidGameState
    );
    // What if game just finished but status not updated yet?
}

// NO STATE VALIDATION in refund_wager_handler
pub fn refund_wager_handler(/* ... */) -> Result<()> {

    // No check: Can refund completed games?
    // No check: Can refund games in progress?
    // No check: Can refund already refunded games?
    
    for player in players {
        anchor_spl::token::transfer(/* refund */, refund)?;
    }
    
    game_session.status = GameStatus::Completed;  //Wrong final state
}
```

### Attack Scenarios
```rust
// Scenario 1: Join completed game
join_user_handler(ctx, "finished_game", 0)?;  // Should fail but doesn't

// Scenario 2: Refund game in progress
refund_wager_handler(ctx, "active_game")?;    // Should fail but doesn't

// Scenario 3: Buy spawns in waiting game
pay_to_spawn_handler(ctx, "waiting_game", 0)?; // Partially validated
```

### Impact
- **State Machine Violation:** Games operate in invalid states
- **Logic Corruption:** Operations performed at wrong times
- **Economic Exploitation:** Refunds during active games


---

## **FH-005: Remaining Accounts Design Flaw**

**Severity:** High   
**Location:** Distribution and refund functions

### Vulnerable Code
```rust
// COMPLEX VALIDATION: Using remaining_accounts instead of stored data
pub fn distribute_all_winnings_handler(
   ...
) -> Result<()> {
    let game_session = &ctx.accounts.game_session;
    ...
    let players_per_team = game_session.game_mode.players_per_team();
    
    // Get winner list from stored data
    let winning_players = if winning_team == 0 {
        &game_session.team_a.players[0..players_per_team]
    } else {
        &game_session.team_b.players[0..players_per_team]
    };
    
    //But then require remaining_accounts to match
    require!(
        ctx.remaining_accounts.len() >= 2 * players_per_team,
        WagerError::InvalidRemainingAccounts
    );

    for i in 0..players_per_team {
        //Complex indexing into remaining_accounts
        let winner = &ctx.remaining_accounts[i * 2];
        let winner_token_account = &ctx.remaining_accounts[i * 2 + 1];
        
        // Must verify remaining_accounts match stored data
        let winner_pubkey = winner.key();
         require!(
            winner_token_account.owner == winner.key(),
            WagerError::InvalidWinnerTokenAccount
        );
        // Why not just derive the ATA from stored player data?
    }
}
```

### Problems with Current Approach
```rust
// Issues:
// 1. Using remaining accounts in anchor is not advised. 
// 2. Complex validation required for every account
// 3. Easy to pass wrong accounts in wrong order
// 4. Redundant - game already stores all player pubkeys
```

### Impact
- **Manipulation Risk:** Caller can provide incorrect accounts
- **Complexity:** Difficult to validate account ordering
- **Inefficiency:** Using remaining accounts in Anchor is not always the best
- **Error Prone:** Easy to mess up account order


---

## **FH-006: No Authority Validation**

**Severity:** High  
**Location:** Various functions

### Vulnerable Code
```rust
// MISSING AUTHORITY CHECK in record_kill_handler
pub fn record_kill_handler(
...
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // No check: Is the caller authorized to record kills?
    // Missing: game_session.authority validation
    
    game_session.add_kill(killer_team, killer, victim_team, victim)?;
    Ok(())
}

// MINIMAL AUTHORITY CHECK in distribute functions
#[account(
    mut,
    constraint = game_session.authority == game_server.key() @ WagerError::UnauthorizedDistribution,
)]
pub game_session: Account<'info, GameSession>,

// But what about other sensitive operations?
```

### Missing Authority Checks
```rust
// Functions that should validate authority but don't:

// 1. record_kill_handler - anyone can record fake kills
// 2. pay_to_spawn_handler - should verify player authorization
// 3. join_user_handler - basic validation but could be stronger
// 4. Some distribution functions have checks, others don't
```

### Impact
- **Unauthorized Operations:** Non-authorized users can perform sensitive actions
- **Game Manipulation:** Fake kills can be recorded
- **Economic Attacks:** Unauthorized fund movements

---

## **FH-007: Double Spend Vulnerability**

**Severity:** High  
**Location:** `refund_wager_handler()`

### Vulnerable Code
```rust
// NO REFUND TRACKING: Same player can be refunded multiple times
pub fn refund_wager_handler(
  ...
) -> Result<()> {
    let game_session = &ctx.accounts.game_session;
    let players = game_session.get_all_players();

    for player in players {
        ...

        let refund = game_session.session_bet;
        
        // No check: Has this player already been refunded?
        //No tracking: Who has received refunds?
        
        // Find player in remaining_accounts
        ...

        // Transfer refund without any tracking
        anchor_spl::token::transfer(/* ... */, refund)?;
    }

    // Mark as completed but no refund state tracking
    game_session.status = GameStatus::Completed;
    Ok(())
}
```

### Attack Scenario
```rust
// Attack: Call refund function multiple times
// 1. First call: refund_wager_handler() - players get refunded
// 2. Status changes to Completed, but no tracking who was refunded
// 3. If function is called again with same players in remaining_accounts
// 4. Second refund occurs if vault still has funds
// 5. Players receive double refunds
```

### Impact
- **Vault Drainage:** Multiple refunds drain vault
- **Economic Loss:** Platform loses funds
- **Accounting Errors:** Impossible to track who was actually refunded


---

## **FH-008: Kill Recording Missing Validation**

**Severity:** High   
**Location:** `record_kill_handler()`

### Vulnerable Code
```rust
// NO VALIDATION: Anyone can record any kill
pub fn record_kill_handler(
...
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // No validation that killer exists in killer_team
    // No validation that victim exists in victim_team
    // No validation that killer != victim
    // No validation that teams are different
    
    game_session.add_kill(killer_team, killer, victim_team, victim)?;
    Ok(())
}

pub fn add_kill(
   ...
) -> Result<()> {
    // Gets indices but doesn't validate they're correct
    let killer_player_index: usize = self.get_player_index(killer_team, killer)?;
    let victim_player_index: usize = self.get_player_index(victim_team, victim)?;

    // get_player_index might return wrong team's index
    
    require!(
        self.status == GameStatus::InProgress,
        WagerError::GameNotInProgress
    );

    // Records kill without proper validation
    match killer_team {
        0 => self.team_a.player_kills[killer_player_index] += 1,
        1 => self.team_b.player_kills[killer_player_index] += 1,
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    // ... rest of function
}
```

### Attack Scenarios
```rust
// Attack 1: Record fake kills
record_kill_handler(ctx, session_id, 0, fake_killer, 1, fake_victim)?;

// Attack 2: Self-kills (if allowed)
record_kill_handler(ctx, session_id, 0, player_a, 0, player_a)?;

// Attack 3: Wrong team assignment
// Player is actually in team 1, but recorded as team 0 killer
record_kill_handler(ctx, session_id, 0, team1_player, 1, victim)?;

// Attack 4: Non-existent players
record_kill_handler(ctx, session_id, 0, random_pubkey, 1, another_random)?;
```

### Impact
- **Game Statistics Manipulation:** Fake kills inflate scores
- **Economic Exploitation:** Wrong payouts based on false data
- **Game Integrity Loss:** Statistics become meaningless


---

## **FH-009: Vault Seed Security Weakness**

**Severity:** High  
**Location:** Vault PDA derivation

### Vulnerable Code
```rust
// WEAK SEEDS: Only session_id used for vault derivation
#[account(
    init,
    payer = game_server,
    space = 0,
    seeds = [b"vault", session_id.as_bytes()],  // ❌ Only session_id
    bump
)]
pub vault: AccountInfo<'info>,
```

### Security Weakness
```rust
// Problem: Different games could have same session_id
// Game 1 created by Server A: session_id = "game123"
// Game 2 created by Server B: session_id = "game123"
// Both derive to same vault PDA address!

// Current derivation:
// PDA = derive(["vault", "game123"], program_id)

// If two different authorities create games with same session_id,
// they get the same vault address - potential collision
```

### Attack Scenario
```rust
// Scenario 1: Accidental collision
// Authority A creates: create_game("match001", ...)
// Authority B creates: create_game("match001", ...)
// Result: Same vault PDA, potential fund mixing

// Scenario 2: Intentional attack
// Attacker observes session_id pattern
// Creates game with predictable session_id
// Could interfere with legitimate game vaults
```

### Impact
- **PDA Collisions:** Multiple games mapping to same vault
- **Fund Mixing:** Wrong games accessing wrong vaults
- **Security Risk:** Predictable vault addresses


---
