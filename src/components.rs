use bevy::prelude::*;
use crate::resources::SpellType;

// ── Tile / Map ────────────────────────────────────────────────────────────────

#[derive(Component, Clone, Copy)]
pub struct TilePos {
    pub x: usize,
    pub y: usize,
}

// ── Actors ────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Boss;

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Component)]
pub struct Attack(pub i32);

#[derive(Component)]
pub struct Defense(pub i32);

#[derive(Component)]
pub struct AttackCooldown(pub f32);

#[derive(Component)]
pub struct XpReward(pub u32);

// ── Items / Interactables ─────────────────────────────────────────────────────

#[derive(Component, Clone, Copy, Debug)]
pub enum ItemType {
    Weapon,
    Armor,
    Potion,
    Coins(u32),
    SpellScroll(SpellType),
}

#[derive(Component)]
pub struct Item(pub ItemType);

#[derive(Component)]
pub struct Chest;

#[derive(Component)]
pub struct Ladder;

// ── Player facing / swing effect ─────────────────────────────────────────────

/// Last non-zero movement direction — used to aim the swing effect.
#[derive(Component)]
pub struct FacingDirection(pub Vec2);

impl Default for FacingDirection {
    fn default() -> Self {
        Self(Vec2::X) // face right at spawn
    }
}

/// Visual sword-swing animation entity.
#[derive(Component)]
pub struct SwingEffect {
    pub elapsed: f32,
}

impl SwingEffect {
    pub const DURATION: f32 = 0.32;
}

/// Flicker-on-hit animation. Cycles entity colour between normal and flash.
#[derive(Component)]
pub struct DamageFlinch {
    pub timer: f32,
    pub normal_color: Color,
    pub flash_color: Color,
}

impl DamageFlinch {
    pub const DURATION: f32 = 0.45;
}

// ── Spells & projectiles ──────────────────────────────────────────────────────

#[derive(Component)]
pub struct Projectile {
    pub spell:        SpellType,
    pub damage:       i32,
    pub direction:    Vec2,
    pub speed:        f32,
    pub elapsed:      f32,
    pub max_lifetime: f32,
}

/// Damage-over-time applied by Fireball / Venom Cloud.
#[derive(Component)]
pub struct Burning {
    pub timer:          f32,
    pub damage_per_tick: f32,
    pub tick_elapsed:   f32,
}
impl Burning {
    pub const TICK_RATE: f32 = 0.6;
}

/// Speed debuff applied by Ice Shard.
#[derive(Component)]
pub struct Slowed {
    pub timer:  f32,
    pub factor: f32, // velocity multiplier
}

// ── Enemy telegraph ───────────────────────────────────────────────────────────

/// Placed on an enemy when it starts winding up an attack.
/// Damage only lands when the timer reaches zero.
#[derive(Component)]
pub struct WindUp {
    pub timer: f32,
}

impl WindUp {
    pub const DURATION: f32 = 0.5; // seconds the player has to dodge
}

/// Floating "!" entity that tracks an enemy during its wind-up.
#[derive(Component)]
pub struct AttackWarning {
    pub target: Entity,
}

// ── Loot popup ────────────────────────────────────────────────────────────────

/// Floating world-space text that rises and fades after a chest is opened.
#[derive(Component)]
pub struct LootPopup {
    pub elapsed: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl LootPopup {
    pub const DURATION: f32 = 1.4;
}

// ── Markers for cleanup ───────────────────────────────────────────────────────

/// Everything spawned per-level (tiles, actors, HUD)
#[derive(Component)]
pub struct LevelEntity;

/// Everything spawned for a menu screen
#[derive(Component)]
pub struct MenuEntity;

// ── HUD ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct HudHealthText;

#[derive(Component)]
pub struct HudScoreText;

#[derive(Component)]
pub struct HudLevelText;

/// Slot index 0 = most recent pickup. Up to LootLog::MAX_ENTRIES slots spawned.
#[derive(Component)]
pub struct HudLootLogText(pub usize);

#[derive(Component)]
pub struct HudSpellText;

// ── Brain Rot Panel (YouTube Shorts joke) ────────────────────────────────────

#[derive(Component)]
pub struct BrainRotPanel;

#[derive(Component)]
pub struct BrainRotVideoText;

#[derive(Component)]
pub struct BrainRotVideoTitle;

#[derive(Component)]
pub struct BrainRotLikesText;

#[derive(Component)]
pub struct BrainRotCommentText;

#[derive(Component)]
pub struct BrainRotProgressBar;
