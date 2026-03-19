use bevy::prelude::*;

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
