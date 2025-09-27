# Critical Findings

## **FC-001: Incorrect Data Type Size Calculation**

**Severity:** Critical 
**Location:** GameSession space calculation

### Vulnerable Code
```rust
// WRONG: treats u16 as 16 bytes instead of 2 bytes
#[account(
    init,
    payer = game_server,
    space = 8 + 4 + 10 + 32 + 8 + 1 + (2 * (32 * 5 + 16 * 5 + 16 * 5 + 8)) + 1 + 8 + 1 + 1 + 1,
    //                                           ^^^^^^   ^^^^^^
    //                                   Should be 2*5, not 16*5
    seeds = [b"game_session", session_id.as_bytes()],
    bump
)]
pub game_session: Account<'info, GameSession>,

// The issue: 
// pub player_kills: [u16; 5] = 2 * 5 = 10 bytes (not 16 * 5 = 80 bytes)
// pub player_spawns: [u16; 5] = 2 * 5 = 10 bytes (not 16 * 5 = 80 bytes)
```

### Impact
- **Financial:** Deployment will fail or waste 290 bytes rent per game
- **Operational:** Account creation failures at runtime
- **Scalability:** Massive cost inefficiency at scale



---

## **FC-002: Integer Underflow in Spawn System**

**Severity:** Critical  
**Location:** `add_kill()` function

### Vulnerable Code
```rust
// VULNERABLE: No underflow protection
pub fn add_kill(
  ...
) -> Result<()> {
    // ... validation code ...
    
    match victim_team {
        0 => self.team_a.player_spawns[victim_player_index] -= 1, // 0 - 1 = no negative for a u16
        1 => self.team_b.player_spawns[victim_player_index] -= 1, // Same issue
        _ => return Err(error!(WagerError::InvalidTeam)),
    }
    
    Ok(())
}
```

### Exploit Scenario
```rust
// Player starts with 0 spawns (default value)
let mut spawns: u16 = 0;
spawns -= 1;  // Result: 65,535 (because a u16 is non negative, an underflow wraps to max u16 value)
// Player now has unlimited spawns!
```

### Impact
- **Game Breaking:** Players can become invincible because the underflow will max the u16
- **Exploitation:** Easy to trigger, massive impact


---

## **FC-003: Missing Session ID Length Validation**

**Severity:** Critical  
**Location:** All functions accepting session_id parameter

### Vulnerable Code
```rust
// NO VALIDATION: Accepts any length session_id
pub fn create_game_session_handler(
    ctx: Context<CreateGameSession>,
    session_id: String,  //No length check despite max_len(10) in struct
    ...
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    
    // Direct assignment without validation
    game_session.session_id = session_id;  //Could be 1000+ characters
    // ... rest of function
}

// Same issue in ALL session_id functions:
// - join_user_handler
// - pay_to_spawn_handler
// - record_kill_handler
// - distribute_winnings
// - refund_wager_handler
```

### Attack Vector
```rust
// Attacker passes oversized session_id
let malicious_id = "a".repeat(1000);  // 1000 character string
create_game_session_handler(ctx, malicious_id, 100, game_mode)?;
// Result: Transaction fails with "insufficient space" error
```

### Impact
- **DoS Attack:** Easy to trigger transaction failures
- **User Experience:** Cryptic error messages confuse users
- **System Reliability:** Unpredictable failures

---

## **FC-004: Dangerous AccountInfo Usage**

**Severity:** Critical   
**Location:** Vault account definitions

### Vulnerable Code
```rust
//Uses AccountInfo instead of typed Account
#[account(
    init,
    payer = game_server,
    space = 0,  //No space for state tracking
    seeds = [b"vault", session_id.as_bytes()],
    bump
)]
pub vault: AccountInfo<'info>,  //Bypasses all Anchor type safety

// Problems:
// 1. No data structure to track vault state
// 2. No validation of account contents
// 3. No way to verify deposits/withdrawals
// 4. Bypasses Anchor's built-in safety checks
```

### Impact
- **No State Tracking:** Cannot verify vault balances
- **Type Safety Bypass:** Anchor protections disabled
- **Account Substitution:** Wrong accounts could be passed
- **Audit Trail Missing:** No record of vault operations



---

## **FC-005: Vault Balance Reconciliation Missing**

**Severity:** Critical   
**Location:** All distribution functions

### Vulnerable Code
```rust
// NO BALANCE VERIFICATION in distribution functions
pub fn distribute_all_winnings_handler(/* ... */) -> Result<()> {
    // ... distribution logic ...
    
    for i in 0..players_per_team {
        let winning_amount = game_session.session_bet * 2;
        
        // Transfer tokens
        anchor_spl::token::transfer(/* ... */, winning_amount)?;
    }
    
    // NO CHECK: Is vault actually empty?
    // NO VERIFICATION: Did we distribute everything?
    // MISSING: What if vault has extra funds from pay-to-spawn?
    
    game_session.status = GameStatus::Completed;
    Ok(())
}
```

### Impact
- **Stranded Funds:** Tokens permanently locked in vaults
- **Accounting Errors:** Distributions don't match vault balance
- **Economic Exploit:** Players can inflate vaults then claim excess

---


## **FC-006: Missing Duplicate Player Check**

**Severity:** Critical  
**Location:** `join_user_handler()`

### Vulnerable Code
```rust
// NO DUPLICATE CHECK: Same player can join multiple teams
pub fn join_user_handler(...) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;
    let player = ctx.accounts.user.key();
    
    //NO CHECK: Is this player already in the game?
    let empty_index = game_session.get_player_empty_slot(team)?;
    
    let selected_team = if team == 0 {
        &mut game_session.team_a
    } else {
        &mut game_session.team_b
    };
    
    // Player added without duplicate validation
    selected_team.players[empty_index] = player.key();  // Could be duplicate
    // ... rest of function
}
```

### Attack Scenario
```rust
// Attacker joins both teams in same game
// Step 1: Join Team A
join_user_handler(ctx, "game123", 0)?;  // Player joins team 0

// Step 2: Join Team B (same player, same game)
join_user_handler(ctx, "game123", 1)?;  // Same player joins team 1

// Result: Attacker controls both sides, guaranteed win
```

### Impact
- **Game Manipulation:** Single player controls outcome
- **Economic Exploit:** Guaranteed wins for attackers
- **Platform Integrity:** Destroys fair play


---

## **FC-007: Code Struct Space Implementation**

**Severity:** Critical  
**Location:** All struct definitions

### Current Problem
```rust
// MANUAL CALCULATION: Error-prone and unreliable
#[account(
    init,
    space = 8 + 4 + 10 + 32 + 8 + 1 + (2 * (32 * 5 + 16 * 5 + 16 * 5 + 8)) + 1 + 8 + 3,
    // ^^^ Manual math leads to errors like FC-001
)]
pub game_session: Account<'info, GameSession>,
```

### Impact
- **Deployment Failures:** Incorrect space calculations prevent deployment
- **Development Inefficiency:** Manual calculations are error-prone
- **Maintenance Issues:** Changes require recalculating all sizes

---
