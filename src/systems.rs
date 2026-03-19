use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::{
    components::*,
    level_gen::generate_level,
    resources::*,
    GameState,
};

// ── Generic cleanup ───────────────────────────────────────────────────────────

pub fn cleanup_entities<T: Component>(
    mut commands: Commands,
    query: Query<Entity, With<T>>,
) {
    for e in &query {
        commands.entity(e).despawn_recursive();
    }
}

// ── One-time startup ──────────────────────────────────────────────────────────

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LEVEL SETUP  (OnEnter Playing)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup_level(
    mut commands: Commands,
    mut game_map: ResMut<GameMap>,
    current_level: Res<CurrentLevel>,
    player_stats: Res<PlayerStats>,
) {
    game_map.reset();

    let data = generate_level(current_level.0);
    game_map.tiles = data.tiles;
    game_map.rooms = data.rooms;

    // ── Tile entities ─────────────────────────────────────────────────────────
    let mut tile_entities = vec![Entity::PLACEHOLDER; MAP_WIDTH * MAP_HEIGHT];

    for gy in 0..MAP_HEIGHT {
        for gx in 0..MAP_WIDTH {
            let idx = gy * MAP_WIDTH + gx;
            let tile = game_map.tiles[idx];
            let world = grid_to_world(gx, gy);

            let mut ec = commands.spawn((
                Text2d::new(" "),
                TextFont { font_size: TILE_SIZE, ..default() },
                TextColor(Color::NONE),
                Transform::from_xyz(world.x, world.y, 0.0),
                TilePos { x: gx, y: gy },
                LevelEntity,
            ));

            // Add a physics collider only for walls that border at least one floor tile
            if tile == TileType::Wall {
                let mut borders_floor = false;
                'outer: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = gx as i32 + dx;
                        let ny = gy as i32 + dy;
                        if nx >= 0
                            && ny >= 0
                            && nx < MAP_WIDTH as i32
                            && ny < MAP_HEIGHT as i32
                        {
                            let ni = ny as usize * MAP_WIDTH + nx as usize;
                            if game_map.tiles[ni] == TileType::Floor {
                                borders_floor = true;
                                break 'outer;
                            }
                        }
                    }
                }
                if borders_floor {
                    ec.insert((
                        RigidBody::Fixed,
                        Collider::cuboid(TILE_SIZE / 2.0, TILE_SIZE / 2.0),
                    ));
                }
            }

            tile_entities[idx] = ec.id();
        }
    }

    game_map.tile_entities = tile_entities;

    // ── Player ────────────────────────────────────────────────────────────────
    let pw = grid_to_world(data.player_start.0, data.player_start.1);
    // Split into nested tuples — Bevy Bundle impls only go up to 15 elements.
    commands.spawn((
        (
            Text2d::new("@"),
            TextFont { font_size: TILE_SIZE, ..default() },
            TextColor(Color::srgb(1.0, 1.0, 0.0)),
            Transform::from_xyz(pw.x, pw.y, 2.0),
            Player,
            Health::new(player_stats.max_hp),
            Attack(player_stats.attack),
            Defense(player_stats.defense),
            AttackCooldown(0.0),
        ),
        (
            RigidBody::Dynamic,
            Velocity::zero(),
            Collider::cuboid(TILE_SIZE / 2.0 - 2.0, TILE_SIZE / 2.0 - 2.0),
            GravityScale(0.0),
            Damping { linear_damping: 25.0, angular_damping: 1.0 },
            LockedAxes::ROTATION_LOCKED,
            FacingDirection::default(),
            LevelEntity,
        ),
    ));

    // ── Enemies & Boss ────────────────────────────────────────────────────────
    for spawn in &data.enemies {
        let ew = grid_to_world(spawn.x, spawn.y);
        let (ch, col) = if spawn.is_boss {
            ("B", Color::srgb(1.0, 0.15, 0.15))
        } else {
            ("E", Color::srgb(1.0, 0.45, 0.45))
        };

        let mut ec = commands.spawn((
            (
                Text2d::new(ch),
                TextFont { font_size: TILE_SIZE, ..default() },
                TextColor(col),
                Transform::from_xyz(ew.x, ew.y, 2.0),
                Enemy,
                Health::new(spawn.hp),
                Attack(spawn.attack),
                Defense(spawn.defense),
                AttackCooldown(0.0),
            ),
            (
                XpReward(spawn.xp),
                RigidBody::Dynamic,
                Velocity::zero(),
                Collider::cuboid(TILE_SIZE / 2.0 - 2.0, TILE_SIZE / 2.0 - 2.0),
                GravityScale(0.0),
                Damping { linear_damping: 25.0, angular_damping: 1.0 },
                LockedAxes::ROTATION_LOCKED,
                LevelEntity,
            ),
        ));

        if spawn.is_boss {
            ec.insert(Boss);
        }
    }

    // ── Chests ────────────────────────────────────────────────────────────────
    for &(cx, cy) in &data.chests {
        let cw = grid_to_world(cx, cy);
        commands.spawn((
            Text2d::new("C"),
            TextFont { font_size: TILE_SIZE, ..default() },
            TextColor(Color::srgb(0.85, 0.65, 0.15)),
            Transform::from_xyz(cw.x, cw.y, 1.5),
            Chest,
            LevelEntity,
        ));
    }

    // ── Ladder (hidden until boss dies) ───────────────────────────────────────
    let lw = grid_to_world(data.ladder_pos.0, data.ladder_pos.1);
    commands.spawn((
        Text2d::new(">"),
        TextFont { font_size: TILE_SIZE, ..default() },
        TextColor(Color::srgb(0.2, 1.0, 0.3)),
        Transform::from_xyz(lw.x, lw.y, 1.5),
        Ladder,
        Visibility::Hidden,
        LevelEntity,
    ));

    // ── HUD ───────────────────────────────────────────────────────────────────
    spawn_hud(&mut commands, current_level.0, player_stats.hp, player_stats.max_hp);
}

// ── Between-level transition  (OnEnter LevelTransition) ──────────────────────

pub fn transition_level(
    mut next_state: ResMut<NextState<GameState>>,
    mut current_level: ResMut<CurrentLevel>,
    mut player_stats: ResMut<PlayerStats>,
) {
    current_level.0 += 1;
    player_stats.hp = (player_stats.hp + 8).min(player_stats.max_hp);
    next_state.set(GameState::Playing);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GAMEPLAY SYSTEMS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn tick_cooldowns(time: Res<Time>, mut q: Query<&mut AttackCooldown>) {
    let dt = time.delta_secs();
    for mut cd in &mut q {
        cd.0 = (cd.0 - dt).max(0.0);
    }
}

// ── Player input ──────────────────────────────────────────────────────────────

pub fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Velocity, &mut FacingDirection), With<Player>>,
) {
    let Ok((mut vel, mut facing)) = player_q.get_single_mut() else {
        return;
    };

    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }

    if dir != Vec2::ZERO {
        vel.linvel = dir.normalize() * PLAYER_SPEED;
        facing.0 = dir.normalize();
    } else {
        vel.linvel = Vec2::ZERO;
    }
}

// ── Enemy AI ──────────────────────────────────────────────────────────────────

pub fn enemy_ai(
    player_q: Query<&Transform, With<Player>>,
    mut enemy_q: Query<(&Transform, &mut Velocity), (With<Enemy>, Without<Player>)>,
) {
    let Ok(pt) = player_q.get_single() else {
        return;
    };
    let player_pos = pt.translation.truncate();

    for (et, mut vel) in &mut enemy_q {
        let ep = et.translation.truncate();
        let dist = ep.distance(player_pos);

        if dist > ENEMY_CHASE_RADIUS || dist < ATTACK_RANGE * 0.75 {
            vel.linvel = Vec2::ZERO;
        } else {
            vel.linvel = (player_pos - ep).normalize_or_zero() * ENEMY_SPEED;
        }
    }
}

// ── Combat ────────────────────────────────────────────────────────────────────

pub fn combat_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<
        (&Transform, &mut Health, &Attack, &Defense, &mut AttackCooldown, &FacingDirection),
        With<Player>,
    >,
    mut enemy_q: Query<
        (Entity, &Transform, &mut Health, &Attack, &Defense, &mut AttackCooldown, Option<&Boss>),
        (With<Enemy>, Without<Player>),
    >,
    mut score: ResMut<GameScore>,
    mut game_map: ResMut<GameMap>,
    mut player_stats: ResMut<PlayerStats>,
) {
    let Ok((pt, mut p_hp, p_atk, p_def, mut p_cd, facing)) = player_q.get_single_mut() else {
        return;
    };
    let player_pos = pt.translation.truncate();
    let mut rng = rand::thread_rng();

    // Consume the attack input once and check cooldown.
    let pressed = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyZ);
    let can_attack = pressed && p_cd.0 <= 0.0;

    if can_attack {
        // Spawn the rotating cross swing effect in the facing direction.
        let origin = player_pos + facing.0 * TILE_SIZE * 1.4;
        commands.spawn((
            Text2d::new("+"),
            TextFont { font_size: TILE_SIZE * 1.2, ..default() },
            TextColor(Color::srgb(1.0, 1.0, 0.55)),
            Transform::from_xyz(origin.x, origin.y, 3.0),
            SwingEffect { elapsed: 0.0 },
            LevelEntity,
        ));
        p_cd.0 = ATTACK_COOLDOWN_SECS;
    }

    for (e_ent, et, mut e_hp, e_atk, e_def, mut e_cd, boss) in &mut enemy_q {
        let dist = player_pos.distance(et.translation.truncate());

        if dist < ATTACK_RANGE {
            // Player strike — hits all enemies in range on the same keypress.
            if can_attack {
                let dmg = (p_atk.0 - e_def.0 + rng.gen_range(0..=3)).max(1);
                e_hp.current -= dmg;
            }
            // Enemy strike — automatic.
            if e_cd.0 <= 0.0 {
                let dmg = (e_atk.0 - p_def.0 + rng.gen_range(0..=2)).max(1);
                p_hp.current -= dmg;
                e_cd.0 = ATTACK_COOLDOWN_SECS + 0.25;
            }
        }

        if e_hp.current <= 0 {
            if boss.is_some() {
                game_map.boss_dead = true;
                score.score += 150;
            } else {
                score.score += 10;
            }
            score.kills += 1;
            commands.entity(e_ent).despawn_recursive();
        }
    }

    player_stats.hp = p_hp.current;
}

// ── Swing effect animation ────────────────────────────────────────────────────

pub fn update_swing_effects(
    mut commands: Commands,
    time: Res<Time>,
    player_q: Query<(&Transform, &FacingDirection), With<Player>>,
    mut q: Query<(Entity, &mut SwingEffect, &mut Transform, &mut Text2d, &mut TextColor), Without<Player>>,
) {
    // Snapshot player state once — the swing effect tracks the player as they move.
    let (player_pos, facing_dir) = player_q
        .get_single()
        .map(|(t, f)| (t.translation.truncate(), f.0))
        .unwrap_or((Vec2::ZERO, Vec2::X));

    // Perpendicular to facing — used to arc the effect sideways like a sweep.
    let perp = Vec2::new(-facing_dir.y, facing_dir.x);

    for (entity, mut effect, mut transform, mut text, mut color) in &mut q {
        effect.elapsed += time.delta_secs();
        let t = (effect.elapsed / SwingEffect::DURATION).clamp(0.0, 1.0);

        if t >= 1.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        // Arc: sweep from one side through centre to the other (t 0→1 = +perp→−perp).
        let arc_offset = perp * (1.0 - t * 2.0) * TILE_SIZE * 0.65;
        let pos = player_pos + facing_dir * TILE_SIZE * 1.4 + arc_offset;
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        // Cycle +/x to suggest the cross spinning.
        let frame = (effect.elapsed / 0.07) as usize;
        *text = Text2d::new(if frame % 2 == 0 { "+" } else { "x" });

        // Fade out over the last 35% of the animation.
        let alpha = if t > 0.65 { 1.0 - (t - 0.65) / 0.35 } else { 1.0 };
        *color = TextColor(Color::srgba(1.0, 1.0, 0.55, alpha));
    }
}

// ── Chest / Item pickup ───────────────────────────────────────────────────────

pub fn check_item_pickup(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    chest_q: Query<(Entity, &Transform), With<Chest>>,
    item_q: Query<(Entity, &Transform, &Item)>,
    mut player_stats: ResMut<PlayerStats>,
    mut score: ResMut<GameScore>,
    mut player_health_q: Query<&mut Health, With<Player>>,
) {
    let Ok(pt) = player_q.get_single() else {
        return;
    };
    let pp = pt.translation.truncate();
    let mut rng = rand::thread_rng();

    for (ce, ct) in &chest_q {
        if pp.distance(ct.translation.truncate()) < TILE_SIZE * 0.9 {
            apply_random_loot(&mut player_stats, &mut score, &mut rng, &mut player_health_q);
            commands.entity(ce).despawn_recursive();
        }
    }

    for (ie, it, item) in &item_q {
        if pp.distance(it.translation.truncate()) < TILE_SIZE * 0.9 {
            apply_item(&item.0, &mut player_stats, &mut score, &mut player_health_q);
            commands.entity(ie).despawn_recursive();
        }
    }
}

fn apply_random_loot(
    stats: &mut PlayerStats,
    score: &mut GameScore,
    rng: &mut impl Rng,
    health_q: &mut Query<&mut Health, With<Player>>,
) {
    match rng.gen_range(0u32..4) {
        0 => { stats.attack += 1; score.score += 5; }
        1 => { stats.defense += 1; score.score += 5; }
        2 => {
            let heal = rng.gen_range(6..=12i32);
            heal_player(stats, health_q, heal);
            score.score += 5;
        }
        _ => {
            let coins = rng.gen_range(5u32..=25);
            score.score += coins;
        }
    }
}

fn apply_item(
    item: &ItemType,
    stats: &mut PlayerStats,
    score: &mut GameScore,
    health_q: &mut Query<&mut Health, With<Player>>,
) {
    match item {
        ItemType::Weapon => { stats.attack += 1; score.score += 5; }
        ItemType::Armor  => { stats.defense += 1; score.score += 5; }
        ItemType::Potion => { heal_player(stats, health_q, 10); score.score += 5; }
        ItemType::Coins(n) => { score.score += n; }
    }
}

fn heal_player(
    stats: &mut PlayerStats,
    health_q: &mut Query<&mut Health, With<Player>>,
    amount: i32,
) {
    stats.hp = (stats.hp + amount).min(stats.max_hp);
    if let Ok(mut hp) = health_q.get_single_mut() {
        hp.current = stats.hp;
    }
}

// ── Ladder / progression ──────────────────────────────────────────────────────

pub fn check_ladder(
    player_q: Query<&Transform, With<Player>>,
    mut ladder_q: Query<(&Transform, &mut Visibility), With<Ladder>>,
    game_map: Res<GameMap>,
    current_level: Res<CurrentLevel>,
    mut score: ResMut<GameScore>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(pt) = player_q.get_single() else {
        return;
    };
    let pp = pt.translation.truncate();

    for (lt, mut vis) in &mut ladder_q {
        if game_map.boss_dead {
            *vis = Visibility::Visible;
        }
        if game_map.boss_dead && pp.distance(lt.translation.truncate()) < TILE_SIZE * 0.9 {
            score.score += 50;
            if current_level.0 >= NUM_LEVELS {
                next_state.set(GameState::Victory);
            } else {
                next_state.set(GameState::LevelTransition);
            }
        }
    }
}

// ── Death check ───────────────────────────────────────────────────────────────

pub fn check_death(
    player_q: Query<&Health, With<Player>>,
    mut player_stats: ResMut<PlayerStats>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(hp) = player_q.get_single() else {
        return;
    };
    player_stats.hp = hp.current;
    if hp.current <= 0 {
        next_state.set(GameState::GameOver);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  RENDERING / FOG OF WAR
// ═══════════════════════════════════════════════════════════════════════════════

pub fn update_fog_of_war(
    player_q: Query<&Transform, With<Player>>,
    mut game_map: ResMut<GameMap>,
) {
    let Ok(pt) = player_q.get_single() else {
        return;
    };
    let (px, py) = world_to_grid(pt.translation.truncate());

    // Demote visible → explored
    for f in game_map.fog.iter_mut() {
        if *f == FogState::Visible {
            *f = FogState::Explored;
        }
    }

    // Reveal tiles inside view circle with LoS
    for dy in -VIEW_RADIUS..=VIEW_RADIUS {
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            let tx = px + dx;
            let ty = py + dy;
            if tx < 0 || ty < 0 || tx >= MAP_WIDTH as i32 || ty >= MAP_HEIGHT as i32 {
                continue;
            }
            if dx * dx + dy * dy > VIEW_RADIUS * VIEW_RADIUS {
                continue;
            }
            if line_of_sight(&game_map, px, py, tx, ty) {
                let idx = ty as usize * MAP_WIDTH + tx as usize;
                game_map.fog[idx] = FogState::Visible;
            }
        }
    }
}

fn line_of_sight(map: &GameMap, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    loop {
        if x == x1 && y == y1 {
            return true;
        }
        if (x != x0 || y != y0) && map.tile_at(x, y) == TileType::Wall {
            return false;
        }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x += sx; }
        if e2 < dx  { err += dx; y += sy; }
    }
}

pub fn update_tile_rendering(
    game_map: Res<GameMap>,
    mut tile_q: Query<(&TilePos, &mut Text2d, &mut TextColor)>,
) {
    for (tp, mut text, mut color) in &mut tile_q {
        let idx = game_map.idx(tp.x, tp.y);
        match game_map.fog[idx] {
            FogState::Hidden => {
                *text = Text2d::new(" ");
                *color = TextColor(Color::NONE);
            }
            FogState::Explored => {
                let ch = if game_map.tiles[idx] == TileType::Wall { "#" } else { "." };
                *text = Text2d::new(ch);
                *color = TextColor(Color::srgb(0.18, 0.18, 0.22));
            }
            FogState::Visible => {
                let (ch, c) = if game_map.tiles[idx] == TileType::Wall {
                    ("#", Color::srgb(0.55, 0.55, 0.65))
                } else {
                    (".", Color::srgb(0.28, 0.28, 0.36))
                };
                *text = Text2d::new(ch);
                *color = TextColor(c);
            }
        }
    }
}

/// Hide game entities when their tile is in fog.
/// Excludes: tiles (TilePos), player (Player), UI nodes (Node), sprites (Sprite).
pub fn update_entity_visibility(
    game_map: Res<GameMap>,
    mut q: Query<
        (&Transform, &mut Visibility),
        (
            With<LevelEntity>,
            Without<TilePos>,
            Without<Player>,
            Without<Node>,
            Without<Sprite>,
        ),
    >,
) {
    for (t, mut vis) in &mut q {
        let (gx, gy) = world_to_grid(t.translation.truncate());
        if gx < 0 || gy < 0 || gx >= MAP_WIDTH as i32 || gy >= MAP_HEIGHT as i32 {
            *vis = Visibility::Hidden;
            continue;
        }
        let idx = gy as usize * MAP_WIDTH + gx as usize;
        *vis = if game_map.fog[idx] == FogState::Visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HUD
// ═══════════════════════════════════════════════════════════════════════════════

fn spawn_hud(commands: &mut Commands, level: u32, hp: i32, max_hp: i32) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            LevelEntity,
        ))
        .with_children(|root| {
            // ── Top bar ───────────────────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node::default(),
                    Text::new(format!("Floor  {}/{}", level, NUM_LEVELS)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                    HudLevelText,
                ));
                row.spawn((
                    Node::default(),
                    Text::new("Score: 0   Kills: 0"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.82, 0.1)),
                    HudScoreText,
                ));
            });

            // ── Bottom bar ────────────────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node::default(),
                    Text::new(format!("HP  {}/{}   ATK {}   DEF {}", hp, max_hp, 5, 2)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.15, 1.0, 0.45)),
                    HudHealthText,
                ));
                row.spawn((
                    Node::default(),
                    Text::new("WASD/Arrows: Move  |  Space/Z: Attack (when near enemy)  |  Walk over chest: Open  |  Kill Boss -> > appears"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgb(0.45, 0.45, 0.5)),
                ));
            });
        });
}

pub fn update_hud(
    score: Res<GameScore>,
    player_stats: Res<PlayerStats>,
    current_level: Res<CurrentLevel>,
    mut hp_q: Query<
        &mut Text,
        (With<HudHealthText>, Without<HudScoreText>, Without<HudLevelText>),
    >,
    mut score_q: Query<
        &mut Text,
        (With<HudScoreText>, Without<HudHealthText>, Without<HudLevelText>),
    >,
    mut level_q: Query<
        &mut Text,
        (With<HudLevelText>, Without<HudHealthText>, Without<HudScoreText>),
    >,
) {
    for mut t in &mut hp_q {
        *t = Text::new(format!(
            "HP  {}/{}   ATK {}   DEF {}",
            player_stats.hp, player_stats.max_hp,
            player_stats.attack, player_stats.defense
        ));
    }
    for mut t in &mut score_q {
        *t = Text::new(format!("Score: {}   Kills: {}", score.score, score.kills));
    }
    for mut t in &mut level_q {
        *t = Text::new(format!("Floor  {}/{}", current_level.0, NUM_LEVELS));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MENUS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.04, 0.06)),
            MenuEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Node::default(),
                Text::new("ROGUE  ADVENTURE"),
                TextFont { font_size: 70.0, ..default() },
                TextColor(Color::srgb(1.0, 0.8, 0.05)),
            ));
            p.spawn((
                Node::default(),
                Text::new("Descend five deadly floors and slay the final boss"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
            p.spawn(Node { height: Val::Px(20.0), ..default() });
            p.spawn((
                Node::default(),
                Text::new("[ PRESS  ENTER  TO  START ]"),
                TextFont { font_size: 30.0, ..default() },
                TextColor(Color::srgb(0.25, 1.0, 0.5)),
            ));
            p.spawn(Node { height: Val::Px(28.0), ..default() });

            for &line in &[
                "WASD / Arrow Keys  --  Move",
                "Get near an enemy, press Space or Z  --  Attack",
                "Walk over a chest  --  Open it  (weapon / armor / potion / coins)",
                "Kill the BOSS (B)  --  Ladder (>) appears",
                "Use the ladder  --  Descend to the next floor",
                "",
                "  @  Player       E  Enemy       B  Boss",
                "  C  Chest        >  Ladder",
            ] {
                p.spawn((
                    Node::default(),
                    Text::new(line),
                    TextFont { font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.56)),
                ));
            }
        });
}

pub fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut player_stats: ResMut<PlayerStats>,
    mut score: ResMut<GameScore>,
    mut current_level: ResMut<CurrentLevel>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        *player_stats = PlayerStats::default();
        *score = GameScore::default();
        *current_level = CurrentLevel(1);
        next_state.set(GameState::Playing);
    }
}

// ── Game Over ─────────────────────────────────────────────────────────────────

pub fn setup_game_over(
    mut commands: Commands,
    score: Res<GameScore>,
    current_level: Res<CurrentLevel>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.94)),
            MenuEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Node::default(),
                Text::new("YOU  DIED"),
                TextFont { font_size: 80.0, ..default() },
                TextColor(Color::srgb(0.9, 0.07, 0.07)),
            ));
            p.spawn((
                Node::default(),
                Text::new(format!("Reached floor  {}", current_level.0)),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.72, 0.72, 0.72)),
            ));
            p.spawn((
                Node::default(),
                Text::new(format!("Final Score:  {}", score.score)),
                TextFont { font_size: 42.0, ..default() },
                TextColor(Color::srgb(1.0, 0.82, 0.1)),
            ));
            p.spawn((
                Node::default(),
                Text::new(format!("Enemies killed:  {}", score.kills)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
            p.spawn(Node { height: Val::Px(20.0), ..default() });
            p.spawn((
                Node::default(),
                Text::new("[ ENTER — Return to Menu ]"),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.4, 0.75, 1.0)),
            ));
        });
}

// ── Victory ───────────────────────────────────────────────────────────────────

pub fn setup_victory(mut commands: Commands, score: Res<GameScore>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 0.04, 0.0)),
            MenuEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Node::default(),
                Text::new("VICTORY!"),
                TextFont { font_size: 80.0, ..default() },
                TextColor(Color::srgb(0.15, 1.0, 0.35)),
            ));
            p.spawn((
                Node::default(),
                Text::new("You have conquered the dungeon!"),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
            p.spawn((
                Node::default(),
                Text::new(format!("Final Score:  {}", score.score)),
                TextFont { font_size: 44.0, ..default() },
                TextColor(Color::srgb(1.0, 0.82, 0.1)),
            ));
            p.spawn((
                Node::default(),
                Text::new(format!("Enemies killed:  {}", score.kills)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
            p.spawn(Node { height: Val::Px(20.0), ..default() });
            p.spawn((
                Node::default(),
                Text::new("[ ENTER — Play Again ]"),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.4, 0.75, 1.0)),
            ));
        });
}

pub fn end_screen_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}
