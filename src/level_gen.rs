use rand::Rng;

use crate::resources::{Room, TileType, MAP_HEIGHT, MAP_WIDTH};

const MIN_ROOM_W: usize = 5;
const MAX_ROOM_W: usize = 11;
const MIN_ROOM_H: usize = 4;
const MAX_ROOM_H: usize = 9;

pub struct EnemySpawn {
    pub x: usize,
    pub y: usize,
    pub is_boss: bool,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub xp: u32,
}

pub struct LevelData {
    pub tiles: Vec<TileType>,
    pub rooms: Vec<Room>,
    pub player_start: (usize, usize),
    pub enemies: Vec<EnemySpawn>,
    pub chests: Vec<(usize, usize)>,
    pub ladder_pos: (usize, usize),
}

pub fn generate_level(level: u32) -> LevelData {
    let mut rng = rand::thread_rng();
    let mut tiles = vec![TileType::Wall; MAP_WIDTH * MAP_HEIGHT];
    let mut rooms: Vec<Room> = Vec::new();

    // ── 1. Place 5 rooms randomly ─────────────────────────────────────────────
    let mut attempts = 0;
    while rooms.len() < 5 && attempts < 300 {
        attempts += 1;
        let w = rng.gen_range(MIN_ROOM_W..=MAX_ROOM_W);
        let h = rng.gen_range(MIN_ROOM_H..=MAX_ROOM_H);
        if w + 2 >= MAP_WIDTH || h + 2 >= MAP_HEIGHT {
            continue;
        }
        let x = rng.gen_range(1..MAP_WIDTH - w - 1);
        let y = rng.gen_range(1..MAP_HEIGHT - h - 1);
        let candidate = Room { x, y, w, h };
        if !rooms.iter().any(|r| r.overlaps(&candidate)) {
            rooms.push(candidate);
        }
    }

    // Fallback: force-place rooms in a grid layout
    while rooms.len() < 5 {
        let i = rooms.len();
        let x = 2 + (i % 3) * 20;
        let y = 2 + (i / 3) * 17;
        let (w, h) = (8, 7);
        if x + w < MAP_WIDTH && y + h < MAP_HEIGHT {
            rooms.push(Room { x, y, w, h });
        } else {
            // Last resort — tiny room
            rooms.push(Room { x: 2 + i * 4, y: 2, w: 4, h: 4 });
        }
    }

    // ── 2. Carve rooms into tiles ─────────────────────────────────────────────
    for room in &rooms {
        for ry in room.y..room.y + room.h {
            for rx in room.x..room.x + room.w {
                if rx < MAP_WIDTH && ry < MAP_HEIGHT {
                    tiles[ry * MAP_WIDTH + rx] = TileType::Floor;
                }
            }
        }
    }

    // ── 3. Connect rooms with L-shaped corridors ──────────────────────────────
    for i in 0..rooms.len() - 1 {
        let (ax, ay) = rooms[i].center();
        let (bx, by) = rooms[i + 1].center();
        carve_corridor(&mut tiles, ax, ay, bx, ay); // horizontal
        carve_corridor(&mut tiles, bx, ay, bx, by); // vertical
    }

    // ── 4. Populate rooms ─────────────────────────────────────────────────────
    let (px, py) = rooms[0].center();
    let mut enemies: Vec<EnemySpawn> = Vec::new();
    let mut chests: Vec<(usize, usize)> = Vec::new();

    // Rooms 1–3: enemies or chests
    for i in 1..4usize.min(rooms.len().saturating_sub(1)) {
        let room = &rooms[i];
        if rng.gen_bool(0.5) {
            let count = rng.gen_range(1..=3usize);
            for _ in 0..count {
                let ex = rng.gen_range(room.x + 1..room.x + room.w - 1);
                let ey = rng.gen_range(room.y + 1..room.y + room.h - 1);
                enemies.push(make_enemy(ex, ey, false, level));
            }
        } else {
            let count = rng.gen_range(1..=2usize);
            for _ in 0..count {
                let cx = rng.gen_range(room.x + 1..room.x + room.w - 1);
                let cy = rng.gen_range(room.y + 1..room.y + room.h - 1);
                chests.push((cx, cy));
            }
        }
    }

    // Room index 3 (4th room): mixed content
    if rooms.len() >= 5 {
        let room = &rooms[3];
        let mix = rng.gen_range(1..=3usize);
        for _ in 0..mix {
            if rng.gen_bool(0.5) {
                let ex = rng.gen_range(room.x + 1..room.x + room.w - 1);
                let ey = rng.gen_range(room.y + 1..room.y + room.h - 1);
                enemies.push(make_enemy(ex, ey, false, level));
            } else {
                let cx = rng.gen_range(room.x + 1..room.x + room.w - 1);
                let cy = rng.gen_range(room.y + 1..room.y + room.h - 1);
                chests.push((cx, cy));
            }
        }
    }

    // Room 4 (last): boss + ladder
    let boss_room = rooms.last().unwrap();
    let (bx, by) = boss_room.center();
    enemies.push(make_enemy(bx, by, true, level));

    let lx = (boss_room.x + boss_room.w - 2).min(MAP_WIDTH - 2);
    let ly = (boss_room.y + boss_room.h - 2).min(MAP_HEIGHT - 2);
    tiles[ly * MAP_WIDTH + lx] = TileType::Floor;

    LevelData {
        tiles,
        rooms,
        player_start: (px, py),
        enemies,
        chests,
        ladder_pos: (lx, ly),
    }
}

fn carve_corridor(tiles: &mut Vec<TileType>, x0: usize, y0: usize, x1: usize, y1: usize) {
    let (mut cx, mut cy) = (x0 as i32, y0 as i32);
    let (tx, ty) = (x1 as i32, y1 as i32);
    loop {
        if cx >= 0 && cy >= 0 && cx < MAP_WIDTH as i32 && cy < MAP_HEIGHT as i32 {
            tiles[cy as usize * MAP_WIDTH + cx as usize] = TileType::Floor;
        }
        if cx == tx && cy == ty {
            break;
        }
        if cx != tx {
            cx += if cx < tx { 1 } else { -1 };
        } else if cy != ty {
            cy += if cy < ty { 1 } else { -1 };
        }
    }
}

fn make_enemy(x: usize, y: usize, is_boss: bool, level: u32) -> EnemySpawn {
    if is_boss {
        EnemySpawn {
            x,
            y,
            is_boss: true,
            hp: 40 + level as i32 * 15,
            attack: 6 + level as i32 * 2,
            defense: 2 + level as i32,
            xp: 100 + level * 30,
        }
    } else {
        EnemySpawn {
            x,
            y,
            is_boss: false,
            hp: 8 + level as i32 * 3,
            attack: 3 + level as i32,
            defense: level as i32 / 2,
            xp: 10 + level * 5,
        }
    }
}
