---
name: gun-rem-math
version: 0.1.0-hypothetical
status: hypothetical
purpose: >-
  Mathematical reasoning skill for analyzing, evaluating, and reproducing the decision-making
  patterns of a Gun Rem player in Deadlock from a VOD, while separating measured VOD evidence,
  current game constants, and model-derived conclusions.
source_vod: https://www.twitch.tv/videos/2894727052
source_title: "Highlight: Deadlock - City Never Sleeps == Ala - alacripose"
hero: Rem
archetype: Gun Rem
primary_outputs:
  - expected damage and burst calculations
  - TTK and survival-margin calculations
  - farm/economy conversion calculations
  - cooldown and ability-window calculations
  - target-selection and engagement EV
  - VOD decision-quality scoring
---

# Gun Rem Math Skill

> **Hypothetical skill file:** this is a mathematical operating model for the gameplay represented by the supplied Gun Rem VOD. It is deliberately written as a *skill* rather than a conventional build guide. The goal is to give an agent a way to turn what it sees in a VOD into measurable state, equations, predictions, and decisions.
>
> The source recording is the Twitch highlight/VOD supplied with this skill. The current environment can resolve the public Twitch page/title but cannot expose the recording as a frame-addressable timeline. Therefore this file does **not** invent VOD timestamps or claim that a specific play happened when it was not directly measurable here. VOD-specific measurements should be attached to the `[VOD]` fields described below.

## 0. Epistemic tags

Every important number or conclusion MUST carry one of these provenance labels internally, even if the final answer omits the brackets.

- `[VOD]` — directly measured from the recording: timestamp, position, ammo, visible health, item purchase, cooldown state, kill, miss, hit, retreat, etc.
- `[GAME]` — game constant obtained from current game data / patch documentation.
- `[MODEL]` — computed from `[VOD]` and `[GAME]` values.
- `[ASSUMPTION]` — value introduced only because a required variable is not visible.
- `[INFERENCE]` — a behavioral explanation inferred from repeated observations.

Never upgrade `[ASSUMPTION]` into `[VOD]` merely because it makes a calculation cleaner.

---

# 1. Core doctrine

Gun Rem is not modeled as "a hero who buys gun items."

Gun Rem is modeled as an **economic conversion system**:

```text
map access
    ↓
more farm / more Sinner access / safer collection
    ↓
souls + permanent bonuses + item timings
    ↓
weapon power + sustain + burst windows
    ↓
contact advantage
    ↓
kills / forced retreats / objective damage
    ↓
more map access
```

The central question is therefore not:

> "How much DPS does Rem have?"

It is:

> "Given the current state of the game, how much *winning value per second* is created by spending Rem's time, health, ammunition, cooldowns, souls, and positioning in a particular way?"

A mathematically strong Gun Rem decision can therefore be **not shooting**. Farming, waiting for an ally carrier, preserving Naptime, taking a guaranteed Sinner, reloading before a sleep window, or retreating before the enemy's effective TTK crosses Rem's survival margin can all have higher expected value than holding M1.

---

# 2. State vector

At every meaningful VOD event, construct a state vector.

```text
X(t) = {
    time,
    phase,
    souls,
    net_worth,
    level,
    health,
    max_health,
    stamina,
    ammo,
    ammo_max,
    reload_state,
    fire_ready,

    pillow_ready,
    tag_ready,
    helpers_available,
    naptime_ready,

    weapon_damage_multiplier,
    weapon_damage_flat_or_scaling,
    fire_rate_multiplier,
    magazine_multiplier,
    reload_multiplier,
    range_state,
    resistances,
    healing_modifiers,

    position,
    velocity,
    nearest_cover,
    nearest_ally,
    nearest_enemy,
    visible_enemies,
    hidden_enemies_estimate,
    lane_state,
    objective_state,
    farm_available,
    sinner_available,
    return_distance,

    current_target,
    target_health,
    target_effective_health,
    target_mobility,
    target_escape_route,

    team_state,
    enemy_team_state,
    expected_enemy_damage,
    expected_ally_followup,
    observed_intention,
}
```

A state record should be created at:

1. lane arrival;
2. every purchase/power spike;
3. every kill/death;
4. every Naptime use;
5. every Sinner decision;
6. every major rotation;
7. every committed fight;
8. every retreat caused by danger;
9. objective start/end;
10. any repeated behavior that looks like a policy rather than an isolated action.

For dense fights, sample at event boundaries rather than every video frame.

---

# 3. Time model

Do not confuse video time with game time.

Define:

```text
T_vod  = timestamp in the recording
T_game = game-clock timestamp if visible
ΔT     = elapsed combat/farm time
```

When game time cannot be read, use relative VOD time:

```text
ΔT = T_vod(end) - T_vod(start)
```

When a clip contains editing, cuts, loading screens, or spectator transitions, never treat the visual gap as gameplay elapsed time.

For a game tick system, use:

```text
Δt = Δticks / tick_rate
```

but only when `tick_rate` is known from telemetry. Never silently assume a tick rate.

---

# 4. Current Rem weapon reference constants

These are calibration defaults, not immutable truths. The skill MUST prefer the actual game version/telemetry over this table.

Public data currently reports Rem's Long Night at approximately:

```text
bullet damage       = 16
fire rate            = 3.85 bullets/s
magazine             = 13
reload                = 2.0 s
nominal DPS           = 61.6
published sustained  = 38.7 DPS
falloff starts       ≈ 20 m
falloff reaches end  ≈ 58 m
move speed            = 7.2 m/s
sprint speed          = 4.0 m/s
stamina               = 3
base health           = 680
```

These values are from current public Rem/game-data pages and should be treated as `[GAME]` calibration defaults.

## 4.1 Why nominal DPS is not sustained DPS

A naive calculation gives:

```text
D_nominal = bullet_damage × fire_rate
          = 16 × 3.85
          = 61.6 DPS
```

That is a firing-only figure.

A full magazine contains:

```text
D_mag = 13 × 16 = 208 raw bullet damage
```

Ignoring animation differences and assuming the first shot happens at `t = 0`, 13 shots occupy approximately:

```text
T_13 ≈ (13 - 1) / 3.85
     ≈ 3.117 s
```

Add the 2 s reload:

```text
T_cycle ≈ 3.117 + 2.0
        ≈ 5.117 s
```

A simplified cycle estimate becomes:

```text
D_cycle_estimate ≈ 208 / 5.117
                 ≈ 40.65 DPS
```

The published sustained value is lower, around 38.7 DPS. That difference is exactly why the skill MUST calibrate from observed attack/reload timing when high precision is required.

**Rule:** use the published sustained DPS for broad comparison; use measured VOD timing or game telemetry for exact TTK.

---

# 5. Damage model

## 5.1 Per-shot damage

Define raw weapon damage:

```text
B = base bullet damage
W = multiplicative weapon-damage modifier
F = falloff multiplier
R = target damage/resistance multiplier
H = hit-state multiplier

D_shot = B × W × F × R × H
```

Where:

```text
H = 1                     body hit
H = headshot multiplier   headshot
H = 0                     miss
```

Do not hard-code a headshot multiplier if the current patch or VOD does not establish it.

---

## 5.2 Hit probability

Realized DPS depends more on hit probability than the HUD's firing DPS.

Decompose hit probability:

```text
P_hit = P_aim × P_tracking × P_range × P_visibility × P_target_exposure × P_projectile_connect
```

This is not meant to imply these variables are statistically independent. It is a bookkeeping decomposition.

For a simple model:

```text
E[D_shot] = P_hit × E[D | hit]
```

If headshots occur with probability `P_hs`:

```text
E[D | hit]
    = (1 - P_hs) × D_body
    + P_hs × D_head
```

Therefore:

```text
E[DPS_firing]
    = fire_rate × P_hit × E[D | hit]
```

This is the correct place to capture Rem's forgiving weapon behavior. Do not merely say "Rem auto-headshots." Instead, infer a measurable improvement in `P_hs` and/or a reduced aim-error penalty from the VOD/training data.

---

# 6. Range / falloff model

Let `d` be target distance.

Use a calibrated falloff curve:

```text
F(d) =
    1                         d ≤ d_start
    interpolation(d)          d_start < d < d_end
    F_min                     d ≥ d_end
```

For current reference calibration:

```text
d_start ≈ 20 m
d_end   ≈ 58 m
```

The exact curve and floor should come from current game data if exact damage is required.

The important VOD variable is not distance alone. It is **distance × exposure**.

Define:

```text
contact_quality = damage_multiplier / exposure_time
```

A 15 m target visible for 0.35 s may be worse than a 35 m target visible for 3 s if the latter creates a safe sustained firing lane.

---

# 7. Discrete TTK

For target effective health `H_eff` and per-shot damage `D`:

```text
N_shots = ceil(H_eff / D)
```

For `N_shots` shots fired at rate `r`:

```text
T_fire = (N_shots - 1) / r
```

If a reload occurs between shots:

```text
T_TTK = T_fire + N_reloads × T_reload + animation_delay + retarget_delay
```

A target at 780 effective HP with 16 raw body damage would require:

```text
ceil(780 / 16) = 49 shots
```

which is not a practical single-magazine kill. The published Rem reference likewise shows a large body-shot shot count against a generic 780-HP target. This makes itemization, headshots, buffs, burst windows, and target selection part of the math rather than optional extras.

---

# 8. Magazine economics

Define:

```text
A = current ammo
M = max ammo
r = fire rate
q = shots required for kill
```

Approximate firing time until empty:

```text
T_empty(A) ≈ max(0, A - 1) / r
```

A fight is **magazine constrained** when:

```text
q > A
```

A fight is **time constrained** when:

```text
T_required > T_safe_exposure
```

A fight is **reload constrained** when:

```text
T_required > T_safe_exposure
and
A < q
```

This matters enormously around sleep/CC windows.

---

# 9. Naptime as a burst-window equation

Naptime should be modeled as a temporary expansion of the enemy's vulnerability window rather than merely "an ultimate that sleeps people."

Let:

```text
T_cc       = usable incapacitation duration
T_react    = human reaction / recognition delay
T_aim      = time to acquire target
T_swap     = weapon/item transition delay
T_reload   = reload time if a reload occurs
T_escape   = time enemy needs to leave the kill geometry
```

Then:

```text
T_burst = max(0, T_cc - T_react - T_aim - T_swap)
```

Available theoretical shot opportunities:

```text
N_theoretical ≈ 1 + floor(r × T_burst)
```

But actual shots are bounded by:

```text
N_actual ≤ current_ammo + shots_added_by_reload
```

and by the need to remain in a valid firing state.

## 9.1 Example with the current reference values

Public Rem data lists Naptime at roughly a 4.25 s sleep duration in the base ability reference.

With:

```text
r = 3.85 shots/s
T = 4.25 s
```

there are about:

```text
3.85 × 4.25 = 16.36
```

shot intervals available in an idealized uninterrupted window.

But Rem's default magazine is only 13 shots.

Therefore a clean full-magazine burst is the practical single-window bottleneck, not raw firing rate.

This yields a VOD rule:

> **Before a planned Naptime burst, reload cost is part of the ultimate's damage calculation.**

A player who enters the sleep window with 5 rounds has not really acquired a 4.25-second damage window. They have acquired roughly 5 immediate shots plus a reload decision inside that window.

---

# 10. Pillow Toss as cooldown capital

Public current data reports:

```text
Pillow Toss base cooldown         ≈ 25 s
charge delay                       ≈ 8 s
cooldown reduction on valid hit    ≈ 5 s
```

Pillow's important mathematical property is that **one action can create value in three dimensions**:

```text
enemy displacement
+ enemy debuff / disruption
+ cooldown recovery for Tag Along / Naptime
```

Model each valid refund event as:

```text
C_future = max(0, C_future - R_pillow)
```

with:

```text
R_pillow ≈ 5 s
```

Do not assume every visually overlapping target grants a refund. Record `[VOD]` hit events and use the current game implementation for exact counting.

## 10.1 Pillow timing value

The value of a Pillow hit rises sharply when it changes whether a later high-value ability is available.

Define:

```text
V_refund = P(ability becomes available before next fight)
          × V_ability
```

Then:

```text
V_pillow = V_knockback + V_debuff + V_refund - V_missed_opportunity
```

Therefore throwing Pillow "for damage" is often the wrong model.

The higher-order question is:

> Does this Pillow hit create a future Naptime/Tag Along window that is worth more than holding the ability for immediate displacement?

---

# 11. Tag Along as an effective-health transformation

Tag Along provides a burst heal based on missing health followed by regeneration. The exact current coefficients should be read from the active game version.

Define:

```text
H_missing = H_max - H_current
B_heal    = burst coefficient × H_missing + spirit-scaled component
R_heal    = regeneration per second
T_heal    = effective regeneration duration
```

Then:

```text
H_restored = min(H_missing, B_heal + R_heal × T_heal)
```

For combat survival, healing is best interpreted as **effective damage prevented** only after accounting for whether Rem would otherwise die.

Define survival margin:

```text
S_margin = expected_time_to_die - expected_time_to_receive_healing
```

A Tag Along heal with:

```text
S_margin > 0
```

is mostly sustain.

A Tag Along heal that changes:

```text
S_margin < 0  →  S_margin > 0
```

is a **lethal-state conversion** and is worth disproportionately more.

---

# 12. Lil Helpers as parallel-processing resources

Lil Helpers should be modeled as autonomous workers rather than merely another button.

A helper can perform economy or combat-support work while Rem continues another task.

For a task `i`:

```text
V_task_i = reward_i - travel_cost_i - contest_risk_i - delay_i
```

The defining advantage is concurrency.

If Rem can do task `A` while a Helper does task `B`:

```text
V_parallel ≈ V_A + V_B - coordination_cost
```

rather than:

```text
V_serial ≈ V_A + V_B + travel_time_A + travel_time_B
```

That difference is the mathematical reason a farming-oriented Rem can accumulate unusually high economic throughput.

---

# 13. Sinner's Sacrifice model

Current public Rem documentation states that Lil Helpers can be sent to Sinner's Sacrifice and secure the full jackpot as if using a heavy melee, while a player hitting the machine can interrupt the helper.

That means the Sinner decision has at least four terms:

```text
EV_sinner =
    expected_jackpot_value
  + expected_permanent_stat_value
  + expected_economic_value
  - travel_time_cost
  - exposure_cost
  - contest/death risk
```

## 13.1 Permanent buff value

Current Sinner buff tables include long-lived changes such as:

```text
fire rate
max ammo
cooldown reduction
weapon damage
max health
spirit power
bullet resist
spirit resist
move speed
ability range
```

The exact value of a random jackpot should be modeled probabilistically.

Let each buff type `i` have probability `p_i` and combat value `v_i`:

```text
E[V_buff] = Σ p_i × v_i
```

For Gun Rem, weapon damage, fire rate, max ammo, cooldown reduction, and move speed generally have more direct interaction with the archetype than spirit-only value — but the model must allow the VOD to demonstrate otherwise.

## 13.2 Sinner contest threshold

Define:

```text
P_secure = probability the helper finishes safely
P_death  = probability Rem dies during the trip / contest
V_jackpot = jackpot + permanent stat value
V_death   = death cost
```

Then take the Sinner if:

```text
P_secure × V_jackpot - P_death × V_death > V_best_alternative
```

This prevents the common mistake of treating a permanent buff as free.

---

# 14. Soul rate and economic conversion

Track soul income as a rate, not just a total.

```text
S(t)       = current souls
ΔS         = souls gained over interval
SoulRate   = ΔS / Δt
```

Break the rate into sources:

```text
SoulRate =
    lane_farm
  + jungle_farm
  + boxes
  + Sinners
  + assists
  + kills
  + objective income
  + other_income
```

Then measure conversion efficiency:

```text
ConversionEfficiency
    = combat_value_gained / souls_spent
```

and time-to-spike:

```text
T_spike = souls_remaining_to_purchase / expected_soul_rate
```

This gives a much better decision rule than "I have enough souls to buy X."

The real question is:

> What purchase creates the largest increase in winning probability per expected second until the next meaningful event?

---

# 15. Item marginal-value model

For candidate item `j`:

```text
Cost_j = soul cost
ΔDPS_j = change in expected sustained damage
ΔBurst_j = change in damage during the important fight window
ΔEHP_j = change in effective survivability
ΔFarm_j = change in farming speed
ΔMobility_j = change in escape/engage capability
ΔCC_j = change in ability conversion
```

Define weighted utility:

```text
V_j =
    w_DPS × ΔDPS_j
  + w_Burst × ΔBurst_j
  + w_EHP × ΔEHP_j
  + w_Farm × ΔFarm_j
  + w_Mobility × ΔMobility_j
  + w_CC × ΔCC_j
```

Then marginal value per soul:

```text
MVPS_j = V_j / Cost_j
```

But the weights must change with game phase.

Early lane:

```text
w_Farm and w_survival ↑
```

Midgame:

```text
w_Burst, w_Mobility, w_target_access ↑
```

Late game:

```text
w_survival, w_objective_conversion, w_teamfight_reliability ↑
```

A high-DPS item with poor survivability can therefore have low `MVPS` in a game where Rem already kills targets faster than the enemy can survive but dies first to contact.

---

# 16. Fight expected value

For an action `a`:

```text
EV(a) =
    P_kill × V_kill
  + P_assist × V_assist
  + P_objective × V_objective
  + P_force × V_forced_retreat
  + P_space × V_space
  - P_death × V_death
  - P_cooldown × V_cooldown
  - P_reload × V_reload
  - V_opportunity_cost
```

This equation should be evaluated comparatively rather than pretending the probabilities are exact.

Useful ordinal estimates are sufficient:

```text
very low  = 0.05
low       = 0.20
medium    = 0.50
high      = 0.75
very high = 0.90+
```

Do not fabricate decimal precision from subjective visual evidence.

---

# 17. Target selection

Every possible enemy target gets a score.

```text
TargetScore =
    KillProbability × KillValue
  + Exposure × DamageOpportunity
  + CC_synergy
  + Ally_followup
  + Objective_relevance
  + Economy_denial
  - EnemyCounterplay
  - ChaseDistance
  - EscapeRisk
  - LostFarmOpportunity
```

### 17.1 Kill probability

Approximate from:

```text
P_kill ≈ f(
    target_effective_health,
    current_ammo,
    expected_hit_rate,
    range,
    enemy_mobility,
    available_CC,
    ally_followup,
    Rem_survival_margin
)
```

The agent should prefer **high certainty conversions** over visually exciting low-probability chases when both produce similar value.

---

# 18. Engagement geometry

For any enemy contact, estimate:

```text
D_enemy_to_cover = distance(enemy firing position, nearest useful cover)
D_rem_to_cover   = distance(Rem, nearest useful cover)
V_rem            = effective movement speed
```

Approximate escape time:

```text
T_escape ≈ D_rem_to_cover / V_rem + stamina_penalty + input_delay
```

Estimate enemy lethal time:

```text
T_enemy_kill ≈ target_damage_required / expected_enemy_DPS
```

Define:

```text
SurvivalMargin = T_enemy_kill - T_escape
```

Interpretation:

```text
SurvivalMargin >> 0   safe / space available
SurvivalMargin ≈ 0    knife-edge
SurvivalMargin < 0    disengage unless high-value conversion justifies it
```

This is more useful than "I was low HP."

A player can be at 20% health and be safe if `T_enemy_kill` is large; a player at 70% health can be unsafe if the enemy already has a guaranteed burst angle.

---

# 19. Stamina is a budget, not a decoration

Let:

```text
E_stamina(t) = available stamina
R_stamina    = stamina regeneration rate
C_move       = stamina cost of a movement action
```

Then:

```text
E_after = min(E_max, E_before + R_stamina × Δt) - C_move
```

The skill must track **future stamina debt**.

A movement action is bad if it produces:

```text
immediate_gain < probability-weighted future survival loss
```

Example:

```text
Dash now → gain 5 m of distance
but leaves 0 stamina for enemy counter-engage
```

may be worse than walking/peeking for 0 stamina cost.

---

# 20. Trade efficiency

For a trade window:

```text
DamageEfficiency = enemy_damage_dealt / own_damage_received
```

But pure HP ratio is incomplete.

Use:

```text
TradeValue =
    DamageEfficiency
  + HealingAdvantage
  + AmmoAdvantage
  + CooldownAdvantage
  + PositionAdvantage
  + WaveAdvantage
```

A 200-for-150 trade may be excellent if Rem has Tag Along ready and the enemy does not.

A 400-for-100 trade can still be terrible if Rem is forced out of lane while the enemy retains wave control.

---

# 21. Farm pathing as a shortest-time optimization problem

Represent available farm nodes as a graph.

Each node has:

```text
reward_i
respawn_time_i
travel_distance_i
contest_probability_i
completion_time_i
```

For route `R`:

```text
RouteEV(R)
  = Σ reward_i
  - Σ travel_cost_i
  - Σ contest_risk_i
  - opportunity_cost
```

Then optimize:

```text
EV_per_second(R) = RouteEV(R) / RouteTime(R)
```

Lil Helpers alter this graph by turning some nodes into parallel work.

A strong Gun Rem farmer therefore does not merely "farm a lot."

A strong farmer tries to maintain a high:

```text
EconomicVelocity = useful economic value / unit time
```

while preserving the ability to answer important fights.

---

# 22. The farm/fight threshold

At any moment compare two actions:

```text
A = continue farming
B = join/force fight
```

Compute:

```text
EV_farm = immediate farm + expected future item timing value
EV_fight = expected kill/assist/objective value - death/risk cost
```

Choose fight when:

```text
EV_fight > EV_farm
```

But add **time sensitivity**:

```text
FightUrgency = value lost if the fight is not joined now
```

Then:

```text
AdjustedEV_fight = EV_fight + FightUrgency
```

This avoids the classic economic mistake of saying:

> "I'm farming, so farming is always correct."

Some fights have negative value in isolation but extremely high value because failing to arrive causes a tower loss, a teammate death, or a major objective swing.

---

# 23. Power-spike detection

A power spike is not merely a shop purchase.

Define total combat power:

```text
P = f(
    weapon_damage,
    fire_rate,
    magazine,
    reload,
    headshot_frequency,
    effective_health,
    mobility,
    cooldowns,
    CC,
    permanent_buffs,
    ally_synergy
)
```

A purchase is a meaningful spike if:

```text
ΔP / Δtime
```

creates a large jump in expected fight outcome.

The agent should mark:

```text
SPIKE_1 = first lane pressure spike
SPIKE_2 = first reliable solo kill threshold
SPIKE_3 = first reliable teamfight conversion threshold
SPIKE_4 = late-game survivability / objective threshold
```

The actual VOD should determine where these occur.

---

# 24. Contact-state machine

Model Rem's combat as states:

```text
FARMING
   ↓
SCOUTING
   ↓
CONTACT
   ├── favorable → PRESSURE
   ├── uncertain → PROBE
   └── unfavorable → DISENGAGE

PRESSURE
   ├── target vulnerable → BURST
   ├── enemy reinforcements → RESET
   └── target escapes → REASSESS

BURST
   ├── kill secured → CONVERT
   ├── target survives → CONTINUE/RESET
   └── Rem endangered → SURVIVE

CONVERT
   ├── objective available → OBJECTIVE
   ├── farm available → FARM
   └── enemy respawning → MAP_CONTROL
```

Each state has a mathematical objective.

### FARMING

Maximize:

```text
soul_rate + permanent_buff_rate
```

subject to survival.

### SCOUTING

Maximize:

```text
information_gain / exposure
```

### PROBE

Maximize:

```text
information_gain + cooldown value
```

at low death risk.

### BURST

Maximize:

```text
damage / exposed second
```

### SURVIVE

Maximize:

```text
probability(Rem remains alive) × retained economic value
```

### CONVERT

Maximize:

```text
post-kill map value / time to convert
```

---

# 25. Information gain

A good VOD player often changes decisions because of information rather than because of raw mechanics.

Model every observation as reducing uncertainty.

For enemy hypothesis set `E`:

```text
H(E) = -Σ p_i log2(p_i)
```

A new visual cue changes uncertainty:

```text
InformationGain = H_before - H_after
```

Examples:

```text
Seeing two enemies on map
Seeing a previously hidden lane pushed
Seeing an enemy burn a mobility cooldown
Seeing an enemy buy a defensive item
Hearing or observing a fight begin nearby
```

A high-value Gun Rem rotation can therefore be one that seems to "do nothing" but confirms the enemy is absent from a lane before taking a Sinner.

---

# 26. VOD extraction protocol

When analyzing the supplied recording, do not watch it as a story first. Watch it as a dataset.

## Pass A — segmentation

Divide the recording into meaningful episodes:

```text
LANE
FARM
ROTATION
SINNER
GANK
TEAMFIGHT
ESCAPE
DEATH
OBJECTIVE
SHOP
RESET
```

For each episode record:

```text
start_time
end_time
episode_type
souls_start
souls_end
health_start
health_end
ammo_start
ammo_end
cooldowns_start
cooldowns_end
location
nearby allies
nearby enemies
action_taken
```

## Pass B — event marking

Mark every:

```text
shot sequence
reload
Pillow hit
Tag Along cast
Tag Along eject
Helper assignment
Sinner attempt
Sinner success
Sinner interruption
Naptime cast
Naptime sleep
kill
assist
death
major damage taken
purchase
objective interaction
retreat
rotation
```

## Pass C — decision points

A decision point exists when at least two plausible actions are available.

For each:

```text
ObservedState
AvailableActions
ChosenAction
AlternativeBestGuess
ImmediateOutcome
DelayedOutcome
ExpectedValue
DecisionScore
```

## Pass D — repeated policy detection

Do not overfit to one fight.

A policy becomes `[INFERENCE]` only when the same structure appears repeatedly.

Examples:

```text
"After reaching X soul lead, player leaves lane and hunts Sinners."
"Player reloads before most expected Naptime windows."
"Player uses Pillow primarily for cooldown refund rather than raw damage."
"Player will only chase when escape route remains open."
```

---

# 27. VOD-specific measurements to prioritize

Because the supplied recording should become a *behavioral* skill, prioritize variables that can actually be measured visually.

## Mechanical

```text
shots per encounter
hit rate
headshot rate
ammo at engagement start
ammo at kill attempt
number of reloads per fight
reloads performed while safe
Pillow hit rate
Naptime target count
Naptime-to-kill conversion
Tag Along uses while endangered
```

## Economic

```text
souls/minute
sinners/minute
boxes/minute
lane waves/minute
idle travel time
farm abandoned for fights
fight abandoned for farm
purchase timing
souls remaining after purchase
```

## Tactical

```text
distance at first contact
distance at kill
distance at retreat
cover availability
ally distance
enemy count visible
enemy count inferred
escape-route length
stamina remaining at commitment
```

## Strategic

```text
time spent away from team
number of rotations that generated value
number of rotations that generated nothing
post-kill conversion delay
objective conversion rate
risk-adjusted farm rate
```

---

# 28. Headshot / homing model for Long Night

Current Rem documentation describes Long Night as having a subtle projectile-bending/homing effect toward enemies near the crosshair and notes its unusual headshot behavior.

Do not turn that into a binary assumption of guaranteed headshots.

Instead model an aim error `ε`:

```text
ε = angular distance from ideal aim vector
```

Then define a measured assistance function:

```text
A(ε) ∈ [0,1]
```

where `A(0)` is the best alignment and the effect diminishes with larger error.

A simple empirical model can be fit from VOD shots:

```text
P_hit = base_aim_probability + β × A(ε)
```

subject to:

```text
0 ≤ P_hit ≤ 1
```

This makes the skill testable.

Do not claim that the weapon "aims for you." Measure how much visual correction is actually required.

---

# 29. Fight-window compression

Gun Rem becomes more dangerous when several independent advantages overlap:

```text
enemy CC'd
+ enemy movement suppressed
+ enemy isolated
+ Rem full magazine
+ Pillow refund available
+ target in efficient range
+ ally followup present
+ Rem escape path open
```

Call this a **compressed fight window**.

Define:

```text
WindowQuality =
    Exposure × TargetIsolation × AmmoReadiness × CC × AllyFollowup × Safety
```

The product form is intentionally harsh: a single near-zero factor can ruin the window.

A beautiful Naptime setup has little value if:

```text
AmmoReadiness ≈ 0
```

or:

```text
Safety ≈ 0
```

---

# 30. Burst-value formula

For a finite attack window:

```text
BurstDamage ≈
    N_shots
  × P_hit
  × E[D_shot]
  × target_resistance_modifier
```

with:

```text
N_shots = min(
    ammo_available,
    shots_possible_in_window,
    shots_before_forced_reposition
)
```

Then subtract actual lost time:

```text
BurstDamage_real
    = BurstDamage_ideal
    × (1 - aim_pause_fraction)
    × (1 - movement_fraction)
    × (1 - reload_fraction)
```

These fractions should be measured, not invented.

---

# 31. Why "more DPS" can be mathematically worse

Suppose Item A adds 25% more firing DPS.

But the item's opportunity cost causes:

```text
-10% survival probability
-15% time on target
+20% item cost delay
```

Its practical fight value may be lower.

Define realized fight DPS:

```text
DPS_realized
    = DPS_theoretical
    × P_contact
    × P_survive
    × P_hit
```

A 100-DPS build with 0.55 combined realization can produce:

```text
55 effective DPS
```

while a 75-DPS build with 0.85 realization produces:

```text
63.75 effective DPS
```

This is why positioning and survivability are part of Gun Rem's mathematics.

---

# 32. Kill conversion

A kill is not the endpoint.

Measure:

```text
T_death_to_convert = time between kill and next useful conversion
```

Useful conversions include:

```text
walker / guardian pressure
objective
enemy jungle denial
Sinner
wave clear
ally reset
map control
shop timing
```

Define:

```text
KillConversionEfficiency
    = post-kill objective value / T_death_to_convert
```

A player who kills someone and then spends 18 seconds wandering around may have lower strategic value than a player who gets one fewer kill but immediately converts it into an objective.

---

# 33. Death-cost model

Death should be assigned a full opportunity cost.

```text
DeathCost =
    respawn_time_value
  + lost_lane_value
  + lost_farm_value
  + enemy farm value
  + objective exposure
  + cooldown reset cost
  + map-position loss
  + teamfight absence
```

For a snowballing Gun Rem build, `DeathCost` can increase over time because the player's current soul rate and item value become larger.

Therefore:

```text
acceptable_risk_threshold
    should usually fall as Rem's economic lead increases
```

unless the fight is required to win or close the game.

---

# 34. Lead preservation

A useful model for a lead is:

```text
Lead(t) = RemPower(t) - EnemyCarryPower(t)
```

When:

```text
Lead > 0
```

the player should maximize the probability that the lead survives long enough to convert.

When:

```text
Lead >> 0
```

unnecessary coin-flip fights have low value.

This creates the paradoxical rule:

> The stronger Gun Rem becomes, the more mathematical discipline matters.

A lead that is repeatedly spent on 50/50 chases is not actually a lead; it is a temporary high-roll.

---

# 35. Losing-state model

If Gun Rem is behind, recompute the system.

Let:

```text
EnemyLead = enemy_power - Rem_power
```

Then direct M1 fights become less attractive unless Rem has a setup advantage.

Look for:

```text
high-certainty farm
cheap permanent buffs
isolated targets
ally-assisted CC
counter-picks by geometry rather than raw stats
defensive item value
objective trades
wave stabilization
```

The skill must not blindly maintain the same behavior at every game state.

---

# 36. Lane-specific model

During lane, Rem's weapon value is often more concentrated because:

```text
range is constrained
movement lanes are predictable
enemy HP pools are lower
headshot assistance matters more
Tag Along sustains contact
Pillow creates disruption
```

Define lane pressure:

```text
LanePressure =
    hero_damage
  + wave_damage_value
  + deny_value
  + space_value
  + threat_value
  - self_damage
  - ammo_cost
  - overextension_risk
```

A player should not evaluate each shot only by damage.

A non-damaging shot can still increase `LanePressure` by forcing an enemy behind cover and away from a last hit.

---

# 37. Safe aggression

A Gun Rem aggression is classified as **safe aggression** when:

```text
expected_enemy_loss
>
expected_Rem_loss
```

and at least one of these is true:

```text
cover available
ally followup available
Tag Along available
Pillow available
Naptime available
enemy mobility unavailable
```

It is **speculative aggression** when none of the above stabilizers exist.

The VOD evaluator should count both.

Repeated speculative aggression is a strong signal of an unstable policy unless the recording demonstrates unusually high mechanical conversion.

---

# 38. Retreat quality

Retreat should be scored by timing, not outcome alone.

Let:

```text
t_warning = first frame where danger becomes sufficiently probable
t_retreat = first committed movement away from danger
t_contact = first lethal enemy contact
```

Then:

```text
ReactionLead = t_contact - t_retreat
```

A high positive `ReactionLead` generally indicates early recognition.

A retreat that begins only after taking lethal damage is a late retreat even if Rem ultimately survives.

---

# 39. Reload discipline

Track reloads under three categories:

```text
SAFE_RELOAD
    no credible enemy pressure

RISK_RELOAD
    enemy nearby but cover exists

COMBAT_RELOAD
    reload required inside active threat window
```

Define:

```text
ReloadWaste = safe opportunities missed - unnecessary combat reloads avoided
```

The best timing is not always "reload whenever below 13."

It is:

> Have enough bullets for the *next important window* while paying the reload cost at the *cheapest possible time*.

---

# 40. Ability-resource coupling

The agent should treat ability resources as coupled variables.

Example:

```text
Pillow hit
    ↓
Tag/Nap cooldown shortened
    ↓
more future survival/burst
    ↓
more contact opportunities
```

Likewise:

```text
Helper farm
    ↓
more souls / permanent buffs
    ↓
stronger bullets
    ↓
higher farm speed
    ↓
more souls
```

This is a positive feedback loop.

Positive loops should be identified explicitly because they explain why apparently small actions can have compounding value.

---

# 41. Feedback-loop model

Let Gun Rem power evolve as:

```text
dP/dt = a × FarmRate + b × FightConversion + c × PermanentBuffRate - d × DeathRate
```

where coefficients change with game state.

A successful Gun Rem strategy seeks a state where:

```text
FarmRate ↑
FightConversion ↑
PermanentBuffRate ↑
DeathRate ↓
```

simultaneously.

The strongest VOD sequences should therefore show not merely good fights, but **increasing economic velocity followed by increasingly reliable conversions**.

---

# 42. VOD decision scoring

Each meaningful decision gets a 0–100 score.

```text
StateRecognition      0–20
MechanicalExecution   0–20
ExpectedValue         0–20
RiskManagement        0–15
ResourceTiming        0–15
Conversion            0–10
```

### StateRecognition

Did the player correctly identify:

```text
target availability
enemy numbers
ammo
cooldowns
health
escape route
objective timing
```

### MechanicalExecution

Did the execution match the chosen plan?

### ExpectedValue

Was the action better than the plausible alternatives?

### RiskManagement

Was death risk appropriately priced?

### ResourceTiming

Were ammo, health, stamina, Pillow, Tag Along, Helpers, Naptime, and souls used at useful times?

### Conversion

Did the outcome produce another useful state?

---

# 43. Distinguish mechanical errors from strategic errors

## Mechanical error

The player selected a good action but failed to execute.

Examples:

```text
missed burst
missed Pillow
bad aim correction
reload too late
movement error
```

## Strategic error

The player executed the selected action correctly, but the action itself was low-EV.

Examples:

```text
correctly hits all shots on a target that should never have been chased
perfectly executes a doomed Sinner contest
cleanly empties magazine into the wrong target
uses Naptime after the actual conversion window is already gone
```

The evaluator MUST keep these error classes separate.

---

# 44. Hypothetical policy extraction

After the VOD is fully segmented, produce a policy in this form:

```text
POLICY: <name>
TRIGGER:
    observable state condition

ACTION:
    exact sequence

MATH:
    relevant equation(s)

WHY:
    expected-value explanation

FAILURE_CONDITION:
    state in which policy should be abandoned

COUNTERPOLICY:
    enemy behavior that invalidates it

VOD_EVIDENCE:
    timestamps / event IDs
```

Example policy schema:

```text
POLICY: Full-magazine sleep conversion
TRIGGER:
    Naptime ready AND target isolated AND current ammo sufficient AND escape route open

ACTION:
    preload magazine → initiate sleep → acquire highest-value target → fire continuously → disengage before safety margin collapses

MATH:
    burst_damage = shots_in_sleep_window × expected_damage_per_shot

FAILURE_CONDITION:
    current ammo too low, enemy backup entering, or survival margin negative
```

No specific timestamp should be invented until the VOD has been measured.

---

# 45. What to learn from a Gun Rem VOD

The skill should attempt to infer five layers.

## Layer 1 — Mechanics

```text
aim
tracking
headshot conversion
movement
reloads
ability accuracy
```

## Layer 2 — Tactics

```text
target selection
burst timing
CC usage
retreat timing
cover usage
```

## Layer 3 — Economy

```text
farm rate
Sinner frequency
purchase timing
soul conversion
```

## Layer 4 — Strategy

```text
when to farm
when to fight
when to rotate
when to pressure
when to protect a lead
```

## Layer 5 — Policy

```text
repeated rules governing all four layers
```

A true skill.md file should contain Layer 5, not merely a transcript of Layer 1.

---

# 46. What NOT to do

Never:

```text
- infer skill from kills alone
- infer correctness from winning alone
- call nominal DPS "real DPS"
- assume every fight is equal
- assume a missed shot is a bad decision
- assume a kill is a good decision
- ignore reload time
- ignore range/falloff
- ignore health and retreat geometry
- ignore permanent Sinner value
- ignore opportunity cost of farming
- treat souls as an abstract number instead of future power
- treat Helpers as a simple cooldown button
- assume the same build should be optimal in every state
- hard-code current constants without patch provenance
- invent VOD timestamps
```

---

# 47. Minimal online-decision algorithm

For an agent making a live-like decision from visual state:

```python
observe()

state = build_state_vector()

candidates = generate_actions([
    "farm",
    "push",
    "poke",
    "commit",
    "retreat",
    "rotate",
    "sinner",
    "objective",
    "reload",
    "shop",
])

for action in candidates:
    action.ev = expected_value(action, state)
    action.risk = death_risk(action, state)
    action.resource_cost = resource_cost(action, state)
    action.future = future_position_value(action, state)

best = argmax(
    action.ev
    - action.risk * death_cost(state)
    - action.resource_cost
    + action.future
)

execute(best)

observe_outcome()
update_model()
```

The model should update its coefficients after the outcome rather than pretending its initial estimates were exact.

---

# 48. Bayesian-style calibration from the VOD

If the VOD suggests an initial estimate:

```text
P_hit_prior
```

and the observed result is a hit/miss, update it incrementally.

For hit probability:

```text
P_hit_posterior
    ≈ (hits + prior_successes)
      / (shots + prior_trials)
```

Use a Beta prior if implementation details allow:

```text
P_hit ~ Beta(α, β)
```

then:

```text
α' = α + hits
β' = β + misses
```

The point is not statistical elegance. The point is to prevent a skill from concluding after one clip:

> "The player's headshot rate is 92%."

Instead it can say:

> "Observed sample suggests a high headshot-conversion regime; confidence increases as more engagements accumulate."

---

# 49. Counterfactual engine

For each questionable VOD decision, construct at least one counterfactual.

```text
Observed action: A
Counterfactual: B
```

Estimate:

```text
EV(A)
EV(B)
ΔEV = EV(B) - EV(A)
```

Then classify:

```text
ΔEV < -large     observed choice clearly superior
ΔEV ≈ 0          both defensible
ΔEV > 0 small    minor improvement
ΔEV > 0 large    strategic error
```

Do not label an action an error unless a plausible alternative has materially higher expected value.

---

# 50. Example counterfactual: farm vs fight

Suppose:

```text
farm option:
    +500 expected souls over 25 s
    +8% probability of reaching next item before a fight

fight option:
    50% kill chance
    35% death chance
    kill value = 800 equivalent
    death cost = 1000 equivalent
```

Very rough fight EV:

```text
0.50 × 800 - 0.35 × 1000
= 400 - 350
= +50
```

Farm's value might be much larger once its future item timing is included.

But if the fight is protecting a Walker worth another 1,500 equivalent value, the same fight can become strongly positive.

This demonstrates why **context must enter the equation**.

---

# 51. Example counterfactual: Naptime now vs later

Action A: Naptime a visible enemy with 2 teammates nearby.

Action B: hold Naptime for 15 seconds because an isolated carry is likely to appear.

Estimate:

```text
A:
    target_count = 1
    backup_count = 2
    burst_window = good
    kill_probability = medium
    survival = uncertain

B:
    immediate_value = 0
    future_target_probability = high
    future_kill_value = high
```

A should win only when its current fight value exceeds the discounted future value of the ultimate.

This is the mathematical form of **ult patience**.

---

# 52. Example counterfactual: empty-mag chase

Suppose Rem has:

```text
ammo = 3/13
enemy HP = 500
Pillow unavailable
Tag Along unavailable
Naptime unavailable
nearest cover = 18 m away
```

Even if the enemy appears low, the model must recognize:

```text
magazine deficit
+ low ability support
+ reduced kill probability
+ increased reload exposure
```

The correct action may be to reload and re-enter rather than chase.

This is an important Gun Rem principle:

> **Visible enemy HP is not the same thing as killability.**

Killability is a function of enemy HP, Rem's current ammo, expected hit rate, cooldown state, target mobility, and survival margin.

---

# 53. Economic snowball model

Let:

```text
S_t = souls at time t
P_t = combat power at time t
F_t = farm velocity
K_t = successful combat conversion rate
D_t = death penalty rate
```

Then approximate:

```text
S_(t+1) = S_t + F_t + K_t - purchases
P_(t+1) = P_t + g(purchases, sinners, level)
```

A Gun Rem lead can form a positive feedback:

```text
higher P
→ safer farming
→ higher F
→ earlier purchases
→ higher P
```

The VOD should therefore be checked for the **moment this loop begins**.

That moment is often more strategically important than the eventual final scoreboard.

---

# 54. Measuring player style from the VOD

Do not classify the player's style from one scene.

Compute normalized frequencies over the recording:

```text
AggressionRate
    = committed fights / possible contact opportunities

FarmPriority
    = farm time / non-travel available time

UltPatience
    = fraction of high-value Naptime windows preserved for conversion

ReloadDiscipline
    = safe reloads / total reload opportunities

ConversionRate
    = successful objective/map conversions / kills

SinnerRiskRate
    = contested Sinner attempts / total Sinner opportunities

EscapeDiscipline
    = retreats initiated before danger becomes lethal / all retreats
```

These become the behavioral fingerprint of the player.

---

# 55. Behavioral clustering

Once enough decisions are measured, cluster them into patterns such as:

```text
ECONOMIC GUN REM
    high farm velocity
    low unnecessary combat frequency

PRESSURE GUN REM
    high lane contact
    repeated safe poke
    frequent short trades

BURST GUN REM
    low continuous exposure
    high Naptime conversion

GREEDY GUN REM
    high Sinner / farm contest rate
    higher economic velocity
    higher death variance

CARRY GUN REM
    deliberately converts soul lead into sustained teamfight pressure
```

A VOD can contain more than one cluster. The goal is to identify transitions between them.

---

# 56. Transition detector

Detect strategy changes when one or more state variables cross a threshold:

```text
souls lead > threshold
item spike reached
ultimate unlocked
Sinner buff count increased
enemy carry completed item
team lost outer objectives
Rem deaths increased
map became dark / unsafe
```

Represent a transition as:

```text
OLD_POLICY → TRIGGER → NEW_POLICY
```

Example:

```text
LANE_PRESSURE
    → first major item + lane becomes unsafe
    → ECONOMIC_ROTATION
```

This is one of the most valuable things to extract from a long VOD because it converts gameplay into a state machine.

---

# 57. Agent instruction: ask what variable changed

Whenever the observed action changes, the model should ask:

```text
What changed?

position?
health?
ammo?
souls?
item?
cooldown?
enemy visibility?
ally position?
objective timer?
```

Then classify the cause:

```text
mechanical
resource
information
opponent
objective
strategy
```

This reduces narrative hallucination.

---

# 58. Math skill output format

When asked to analyze a VOD segment, return:

```text
## Segment
<timestamp range>

## State
<key state variables>

## Observed action
<what happened>

## Mathematical model
<equations>

## Estimated value
<EV and risk>

## Best alternative
<counterfactual>

## Verdict
<better / neutral / worse>

## Why
<short explanation>

## Reusable policy
<trigger → action → failure condition>
```

For long recordings, additionally return a summary table:

```text
segment | phase | action | EV | risk | conversion | lesson
```

---

# 59. Suggested VOD-derived skill sections after frame access exists

Once the recording can actually be inspected frame-by-frame, append these sections to this file rather than rewriting the mathematical core:

```text
## VOD Findings

### Opening lane
- timestamp
- first purchase
- first trade
- first Sinner interaction

### First power spike
- timestamp
- soul count
- item
- measured DPS change

### First major fight
- target selection
- ammo state
- Pillow state
- Tag Along state
- Naptime state
- outcome

### Midgame economic transition
- souls/min
- farm path
- Sinner frequency
- team participation

### Late-game conversion
- teamfight TTK
- survival margin
- objective conversion

### Repeated policies
- policy A
- policy B
- policy C
```

Do not create these sections with fabricated timestamps.

---

# 60. Recommended quantitative scorecard

At the end of the VOD, calculate:

```text
MechanicsScore          = f(hit rate, headshots, reloads, ability accuracy)
EconomyScore            = f(souls/min, Sinners, farm uptime)
ResourceScore            = f(ammo, stamina, cooldown timing)
TargetScore              = f(target priority, killability estimation)
RiskScore                = f(death rate, retreat timing, contest decisions)
ConversionScore          = f(kill → objective / map value)
FightWindowScore         = f(CC × ammo × isolation × safety × followup)
AdaptationScore          = f(policy changes after state changes)
```

Overall:

```text
GunRemMathScore
    = weighted_average(
        MechanicsScore,
        EconomyScore,
        ResourceScore,
        TargetScore,
        RiskScore,
        ConversionScore,
        FightWindowScore,
        AdaptationScore
      )
```

Weights should be tuned to the purpose of the review. For skill extraction, `AdaptationScore`, `EconomyScore`, and `ConversionScore` should receive substantial weight because those are hardest to infer from ordinary mechanical statistics.

---

# 61. Compact live-use rules

When the model has little time, use these rules in order:

```text
1. Can I die before I can escape?
2. Do I have enough ammo for the actual kill window?
3. Is the target actually killable, not merely low HP?
4. Is Pillow better used now or as cooldown capital?
5. Can Tag Along convert incoming damage into a survivable state?
6. Is Naptime creating a real burst window or merely a CC animation?
7. Can Helpers create value while I do something else?
8. Is this Sinner safer than the best nearby alternative?
9. Will this fight pay more than the farm I am abandoning?
10. After the kill, what is the fastest useful conversion?
11. If I have a lead, am I preserving it?
12. If I am behind, am I taking a high-certainty route back into the game?
```

---

# 62. Final Gun Rem mathematical doctrine

```text
GUN REM IS NOT:
    M1 until someone dies.

GUN REM IS:
    economic acceleration
    × reliable weapon contact
    × temporary CC windows
    × sustain
    × permanent stat accumulation
    × target isolation
    × survival discipline
    × post-fight conversion.
```

The objective is to maximize:

```text
WinningValueRate
    =
      (farm value
       + combat value
       + objective value
       + permanent value
       + information value)
      /
      (time
       + death risk
       + resource expenditure)
```

The strongest policy is the one that repeatedly increases this ratio across changing game states.

The ultimate test of the skill is whether, after observing a new Gun Rem VOD, it can explain:

```text
WHY the player chose the fight,
WHY that target was killable,
WHY that timing was safe,
HOW much ammo / cooldown / health the window actually contained,
WHAT the player gained economically,
WHAT alternative action existed,
WHAT the action caused 10–30 seconds later,
AND which repeated rule should be extracted for future games.
```

That is the difference between a gameplay transcript and a mathematical gameplay skill.

---

# 63. Source registry / calibration notes

The VOD source is the user-supplied Twitch recording:

```text
https://www.twitch.tv/videos/2894727052
```

Useful current Rem/game references consulted while constructing the mathematical defaults include:

- Deadlock.io Rem guide/stat page — current public Rem weapon stats and ability data.
- SteamDB Deadlock Rem page — current public weapon/base-stat snapshot.
- Deadlock Wiki Rem/Pillow Toss/Naptime pages — detailed ability and weapon notes, including Long Night's projectile behavior and Pillow cooldown interaction.
- Valve's May 22, 2026 Deadlock gameplay update — current Rem changes to Pillow Toss, Tag Along, and Naptime.
- Deadlock Sinner's Sacrifice page — jackpot mechanics and permanent buff categories.
- Current community Gun Rem build pages — useful as evidence of the contemporary Gun Rem archetype, but **not** authoritative evidence that the VOD used any particular build.

Important: community builds and Reddit discussions are strategic evidence, not ground truth. The VOD itself should dominate any claim about the player's actual build or behavior once frame-level inspection is available.

---

# 64. Implementation note

This file intentionally keeps game constants and behavioral inference separate.

Recommended future machine-readable companion objects:

```text
vod_events.jsonl
combat_windows.jsonl
purchase_events.jsonl
sinner_events.jsonl
state_snapshots.jsonl
counterfactuals.jsonl
policy_candidates.jsonl
```

Each record should contain:

```json
{
  "timestamp": 0.0,
  "game_time": null,
  "tag": "VOD",
  "phase": "TEAMFIGHT",
  "state": {},
  "action": "N/A",
  "outcome": "N/A",
  "confidence": 0.0
}
```

That structure allows later agent tooling to replace prose claims with auditable observations without rewriting the mathematical doctrine.
