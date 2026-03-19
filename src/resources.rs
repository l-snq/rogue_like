use bevy::prelude::*;
use bevy::text::Font;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const TILE_SIZE: f32 = 18.0;
pub const MAP_WIDTH: usize = 66;
pub const MAP_HEIGHT: usize = 38;
pub const NUM_LEVELS: u32 = 5;
pub const VIEW_RADIUS: i32 = 9;

pub const PLAYER_SPEED: f32 = 180.0;
pub const ENEMY_SPEED: f32 = 65.0;
pub const ENEMY_CHASE_RADIUS: f32 = TILE_SIZE * 10.0;
pub const ATTACK_COOLDOWN_SECS: f32 = 0.75;
pub const ATTACK_RANGE: f32 = TILE_SIZE * 1.6;

pub const SHIELD_DRAIN_RATE: f32 = 28.0;      // stamina / sec while blocking
pub const SHIELD_REGEN_RATE: f32 = 18.0;      // stamina / sec when idle
pub const SHIELD_BREAK_RECOVERY: f32 = 2.2;   // seconds until usable again
pub const SHIELD_BLOCK_RATIO: f32 = 0.15;     // fraction of damage that bleeds through

// ── Coordinate helpers ────────────────────────────────────────────────────────

/// Top-left corner of the map in world space.
pub fn map_origin() -> Vec2 {
    Vec2::new(
        -(MAP_WIDTH as f32 * TILE_SIZE) / 2.0,
        (MAP_HEIGHT as f32 * TILE_SIZE) / 2.0,
    )
}

/// Tile grid position → world-space centre.
pub fn grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let o = map_origin();
    Vec2::new(
        o.x + gx as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        o.y - gy as f32 * TILE_SIZE - TILE_SIZE / 2.0,
    )
}

/// World-space position → tile grid position (may be out of bounds).
pub fn world_to_grid(world: Vec2) -> (i32, i32) {
    let o = map_origin();
    let gx = ((world.x - o.x) / TILE_SIZE).floor() as i32;
    let gy = ((o.y - world.y) / TILE_SIZE).floor() as i32;
    (gx, gy)
}

// ── Tile / Fog types ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum TileType {
    #[default]
    Wall,
    Floor,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum FogState {
    #[default]
    Hidden,
    Explored,
    Visible,
}

// ── Room ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Room {
    pub fn center(&self) -> (usize, usize) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn overlaps(&self, other: &Room) -> bool {
        self.x < other.x + other.w + 2
            && self.x + self.w + 2 > other.x
            && self.y < other.y + other.h + 2
            && self.y + self.h + 2 > other.y
    }
}

// ── GameMap ───────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameMap {
    pub tiles: Vec<TileType>,
    pub fog: Vec<FogState>,
    pub tile_entities: Vec<Entity>,
    pub rooms: Vec<Room>,
    pub width: usize,
    pub height: usize,
    pub boss_dead: bool,
}

impl Default for GameMap {
    fn default() -> Self {
        Self {
            tiles: vec![TileType::Wall; MAP_WIDTH * MAP_HEIGHT],
            fog: vec![FogState::Hidden; MAP_WIDTH * MAP_HEIGHT],
            tile_entities: vec![Entity::PLACEHOLDER; MAP_WIDTH * MAP_HEIGHT],
            rooms: Vec::new(),
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            boss_dead: false,
        }
    }
}

impl GameMap {
    pub fn reset(&mut self) {
        self.tiles.fill(TileType::Wall);
        self.fog.fill(FogState::Hidden);
        self.tile_entities.fill(Entity::PLACEHOLDER);
        self.rooms.clear();
        self.boss_dead = false;
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn tile_at(&self, x: i32, y: i32) -> TileType {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return TileType::Wall;
        }
        self.tiles[self.idx(x as usize, y as usize)]
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.tile_at(x, y) == TileType::Floor
    }

    pub fn set_floor(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let i = self.idx(x, y);
            self.tiles[i] = TileType::Floor;
        }
    }
}

// ── Font resource ─────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

// ── Score / Level resources ───────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct GameScore {
    pub score: u32,
    pub kills: u32,
}

#[derive(Resource)]
pub struct CurrentLevel(pub u32);

impl Default for CurrentLevel {
    fn default() -> Self {
        CurrentLevel(1)
    }
}

#[derive(Resource)]
pub struct PlayerStats {
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub is_blocking: bool,
    pub shield_broken: bool,
    pub shield_recovery: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            hp: 30,
            max_hp: 30,
            attack: 5,
            defense: 2,
            stamina: 100.0,
            max_stamina: 100.0,
            is_blocking: false,
            shield_broken: false,
            shield_recovery: 0.0,
        }
    }
}
