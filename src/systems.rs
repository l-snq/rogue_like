use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::{
    components::*,
    level_gen::generate_level,
    resources::*,
    GameState,
};

// ── Character constants (text-only effects) ───────────────────────────────────

const CH_SWORD: &str = "†";   // U+2020  dagger — swing effect (rotated each frame)

// Sprite tint constants — applied multiplicatively over the PNG texture.
// Color::WHITE = no tint (show texture as-is).
pub const COL_NORMAL:   Color = Color::WHITE;                   // all actors at rest
pub const COL_BLOCKING: Color = Color::srgb(0.4, 0.9,  1.0);  // cyan   — shield up
pub const COL_BROKEN:   Color = Color::srgb(1.0, 0.45, 0.0);  // orange — shield broken

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
    game_font: Res<GameFont>,
    game_textures: Res<GameTextures>,
    loot_log: Res<LootLog>,
) {
    game_map.reset();

    let data = generate_level(current_level.0);
    game_map.tiles = data.tiles;
    game_map.rooms = data.rooms;

    let font = game_font.0.clone();

    // ── Tile entities ─────────────────────────────────────────────────────────
    let mut tile_entities = vec![Entity::PLACEHOLDER; MAP_WIDTH * MAP_HEIGHT];

    for gy in 0..MAP_HEIGHT {
        for gx in 0..MAP_WIDTH {
            let idx = gy * MAP_WIDTH + gx;
            let tile = game_map.tiles[idx];
            let world = grid_to_world(gx, gy);

            let tile_image = if tile == TileType::Wall {
                game_textures.wall.clone()
            } else {
                game_textures.floor.clone()
            };
            let mut ec = commands.spawn((
                Sprite {
                    image: tile_image,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, 0.0),
                Visibility::Hidden,
                TilePos { x: gx, y: gy },
                LevelEntity,
            ));

            if tile == TileType::Wall {
                let mut borders_floor = false;
                'outer: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = gx as i32 + dx;
                        let ny = gy as i32 + dy;
                        if nx >= 0 && ny >= 0 && nx < MAP_WIDTH as i32 && ny < MAP_HEIGHT as i32 {
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
    commands.spawn((
        (
            Sprite {
                image: game_textures.player.clone(),
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
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
        let img = if spawn.is_boss {
            game_textures.boss.clone()
        } else {
            game_textures.enemy.clone()
        };

        let mut ec = commands.spawn((
            (
                Sprite {
                    image: img,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
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
            Sprite {
                image: game_textures.chest.clone(),
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_xyz(cw.x, cw.y, 1.5),
            Chest,
            LevelEntity,
        ));
    }

    // ── Ladder (hidden until boss dies) ───────────────────────────────────────
    let lw = grid_to_world(data.ladder_pos.0, data.ladder_pos.1);
    commands.spawn((
        Sprite {
            image: game_textures.ladder.clone(),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_xyz(lw.x, lw.y, 1.5),
        Ladder,
        Visibility::Hidden,
        LevelEntity,
    ));

    // ── HUD ───────────────────────────────────────────────────────────────────
    spawn_hud(&mut commands, font, current_level.0, player_stats.hp, player_stats.max_hp, player_stats.stamina, player_stats.max_stamina, &loot_log);
}

// ── Level transition ──────────────────────────────────────────────────────────

pub fn transition_level(
    mut next_state: ResMut<NextState<GameState>>,
    mut current_level: ResMut<CurrentLevel>,
    mut player_stats: ResMut<PlayerStats>,
) {
    current_level.0 += 1;
    player_stats.hp = (player_stats.hp + 8).min(player_stats.max_hp);
    player_stats.mana = (player_stats.mana + 30.0).min(player_stats.max_mana);
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

// ── Shield / stamina ──────────────────────────────────────────────────────────

pub fn shield_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_stats: ResMut<PlayerStats>,
) {
    let dt = time.delta_secs();
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if player_stats.shield_broken {
        // Shield is broken — wait out the recovery timer, slowly regen stamina
        player_stats.is_blocking = false;
        player_stats.shield_recovery -= dt;
        player_stats.stamina = (player_stats.stamina + SHIELD_REGEN_RATE * 0.4 * dt)
            .min(player_stats.max_stamina);
        if player_stats.shield_recovery <= 0.0 {
            player_stats.shield_broken = false;
            player_stats.shield_recovery = 0.0;
        }
    } else if shift && player_stats.stamina > 0.0 {
        // Actively blocking — drain stamina
        player_stats.is_blocking = true;
        player_stats.stamina -= SHIELD_DRAIN_RATE * dt;
        if player_stats.stamina <= 0.0 {
            player_stats.stamina = 0.0;
            player_stats.is_blocking = false;
            player_stats.shield_broken = true;
            player_stats.shield_recovery = SHIELD_BREAK_RECOVERY;
        }
    } else {
        // Idle — regenerate stamina
        player_stats.is_blocking = false;
        player_stats.stamina = (player_stats.stamina + SHIELD_REGEN_RATE * dt)
            .min(player_stats.max_stamina);
    }
}

// ── Player colour (reflects shield state) ─────────────────────────────────────

pub fn update_player_color(
    player_stats: Res<PlayerStats>,
    mut q: Query<&mut Sprite, (With<Player>, Without<DamageFlinch>)>,
) {
    let col = if player_stats.shield_broken {
        COL_BROKEN
    } else if player_stats.is_blocking {
        COL_BLOCKING
    } else {
        COL_NORMAL
    };
    for mut sprite in &mut q {
        sprite.color = col;
    }
}

// ── Spell casting ─────────────────────────────────────────────────────────────

pub fn cast_spell(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    game_font: Res<GameFont>,
    mut player_spells: ResMut<PlayerSpells>,
    mut player_stats: ResMut<PlayerStats>,
    mut score: ResMut<GameScore>,
    mut game_map: ResMut<GameMap>,
    player_q: Query<(&Transform, &FacingDirection), With<Player>>,
    mut enemy_q: Query<(Entity, &Transform, &mut Health, Option<&Boss>), With<Enemy>>,
) {
    let dt = time.delta_secs();

    // Mana regen + cooldown tick every frame
    player_stats.mana = (player_stats.mana + MANA_REGEN_RATE * dt).min(player_stats.max_mana);
    player_spells.cooldown = (player_spells.cooldown - dt).max(0.0);

    // Cycle active spell with E
    if keys.just_pressed(KeyCode::KeyE) && player_spells.known.len() > 1 {
        player_spells.active = (player_spells.active + 1) % player_spells.known.len();
    }

    // Cast with Q
    if !keys.just_pressed(KeyCode::KeyQ) { return; }
    if player_spells.cooldown > 0.0 { return; }

    let Ok((pt, facing)) = player_q.get_single() else { return; };
    let player_pos = pt.translation.truncate();

    let spell = player_spells.known[player_spells.active];
    if player_stats.mana < spell.mana_cost() { return; }

    player_stats.mana -= spell.mana_cost();
    player_spells.cooldown = SPELL_COOLDOWN_SECS;

    let (r, g, b) = spell.color();
    let mut rng = rand::thread_rng();

    match spell {
        SpellType::Fireball | SpellType::IceShard => {
            let origin = player_pos + facing.0 * TILE_SIZE;
            commands.spawn((
                Text2d::new(spell.glyph()),
                TextFont { font: game_font.0.clone(), font_size: TILE_SIZE * 0.9, ..default() },
                TextColor(Color::srgb(r, g, b)),
                Transform::from_xyz(origin.x, origin.y, 3.5),
                Projectile {
                    spell,
                    damage: match spell {
                        SpellType::Fireball  => rng.gen_range(8..=12),
                        SpellType::IceShard  => rng.gen_range(5..=8),
                        _                    => 8,
                    },
                    direction: facing.0,
                    speed: TILE_SIZE * 14.0,
                    elapsed: 0.0,
                    max_lifetime: 1.6,
                },
                LevelEntity,
            ));
        }

        SpellType::Lightning => {
            // Find nearest enemy in range (immutable pass)
            let range = TILE_SIZE * 9.0;
            let nearest = enemy_q.iter()
                .filter_map(|(e, et, _, _)| {
                    let d = player_pos.distance(et.translation.truncate());
                    if d < range { Some((e, d)) } else { None }
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(e, _)| e);

            // Mutable pass to apply damage
            if let Some(e_ent) = nearest {
                if let Ok((_, et, mut hp, boss)) = enemy_q.get_mut(e_ent) {
                    let dmg = rng.gen_range(20..=32i32);
                    hp.current -= dmg;
                    commands.entity(e_ent).insert(DamageFlinch {
                        timer: 0.0,
                        normal_color: Color::WHITE,
                        flash_color: Color::srgb(1.0, 1.0, 0.3),
                    });
                    // Spawn a brief flash at enemy position
                    let ep = et.translation;
                    commands.spawn((
                        Text2d::new(spell.glyph()),
                        TextFont { font: game_font.0.clone(), font_size: TILE_SIZE * 1.4, ..default() },
                        TextColor(Color::srgb(r, g, b)),
                        Transform::from_xyz(ep.x, ep.y, 4.0),
                        Projectile {
                            spell,
                            damage: 0,
                            direction: Vec2::ZERO,
                            speed: 0.0,
                            elapsed: 0.0,
                            max_lifetime: 0.25, // just a visual flash
                        },
                        LevelEntity,
                    ));
                    if hp.current <= 0 {
                        if boss.is_some() { game_map.boss_dead = true; score.score += 150; }
                        else { score.score += 10; }
                        score.kills += 1;
                        commands.entity(e_ent).despawn_recursive();
                    }
                }
            }
        }

        SpellType::PoisonCloud => {
            let radius = TILE_SIZE * 3.5;
            // Spawn area cloud glyph at player
            commands.spawn((
                Text2d::new(spell.glyph()),
                TextFont { font: game_font.0.clone(), font_size: TILE_SIZE * 2.2, ..default() },
                TextColor(Color::srgba(r, g, b, 0.85)),
                Transform::from_xyz(player_pos.x, player_pos.y, 3.5),
                Projectile {
                    spell,
                    damage: 0,
                    direction: Vec2::ZERO,
                    speed: 0.0,
                    elapsed: 0.0,
                    max_lifetime: 0.4, // visual only
                },
                LevelEntity,
            ));
            for (e_ent, et, mut hp, boss) in &mut enemy_q {
                if player_pos.distance(et.translation.truncate()) < radius {
                    hp.current -= rng.gen_range(6..=10);
                    commands.entity(e_ent).insert(Burning {
                        timer: 3.5,
                        damage_per_tick: 2.0,
                        tick_elapsed: 0.0,
                    });
                    commands.entity(e_ent).insert(DamageFlinch {
                        timer: 0.0,
                        normal_color: Color::srgb(0.35, 0.9, 0.35),
                        flash_color: Color::srgb(0.1, 1.0, 0.1),
                    });
                    if hp.current <= 0 {
                        if boss.is_some() { game_map.boss_dead = true; score.score += 150; }
                        else { score.score += 10; }
                        score.kills += 1;
                        commands.entity(e_ent).despawn_recursive();
                    }
                }
            }
        }
    }
}

// ── Projectile movement & collision ───────────────────────────────────────────

pub fn update_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    game_map: Res<GameMap>,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform, &mut TextColor)>,
    mut enemy_q: Query<(Entity, &Transform, &mut Health), (With<Enemy>, Without<Projectile>)>,
) {
    let dt = time.delta_secs();

    for (proj_entity, mut proj, mut tf, mut color) in &mut proj_q {
        proj.elapsed += dt;

        // Fade out in last 30% of life
        let t = proj.elapsed / proj.max_lifetime;
        if t >= 1.0 {
            commands.entity(proj_entity).despawn_recursive();
            continue;
        }
        let alpha = if t > 0.7 { 1.0 - (t - 0.7) / 0.3 } else { 1.0 };
        let (r, g, b) = proj.spell.color();
        *color = TextColor(Color::srgba(r, g, b, alpha));

        if proj.speed == 0.0 { continue; } // visual-only flash

        // Move
        let delta = proj.direction * proj.speed * dt;
        tf.translation.x += delta.x;
        tf.translation.y += delta.y;

        // Wall collision
        let pos = tf.translation.truncate();
        let (gx, gy) = world_to_grid(pos);
        if !game_map.is_walkable(gx, gy) {
            commands.entity(proj_entity).despawn_recursive();
            continue;
        }

        // Enemy collision
        let mut hit = false;
        for (e_ent, et, mut e_hp) in &mut enemy_q {
            if pos.distance(et.translation.truncate()) < TILE_SIZE * 0.75 {
                e_hp.current -= proj.damage;
                let (pr, pg, pb) = proj.spell.color();
                commands.entity(e_ent).insert(DamageFlinch {
                    timer: 0.0,
                    normal_color: Color::WHITE,
                    flash_color: Color::srgb(pr, pg, pb),
                });
                match proj.spell {
                    SpellType::Fireball => {
                        commands.entity(e_ent).insert(Burning {
                            timer: 3.0,
                            damage_per_tick: 2.0,
                            tick_elapsed: 0.0,
                        });
                    }
                    SpellType::IceShard => {
                        commands.entity(e_ent).insert(Slowed { timer: 4.0, factor: 0.25 });
                    }
                    _ => {}
                }
                hit = true;
                break;
            }
        }
        if hit { commands.entity(proj_entity).despawn_recursive(); }
    }
}

// ── Status effects (Burning / Slowed) ─────────────────────────────────────────

pub fn update_status_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<
        (Entity, &mut Sprite, &mut Health, Option<&mut Burning>, Option<&mut Slowed>),
        With<Enemy>,
    >,
) {
    let dt = time.delta_secs();
    let t_secs = time.elapsed_secs();

    for (e_ent, mut sprite, mut health, burning, slowed) in &mut q {
        let active_burn = if let Some(mut burn) = burning {
            burn.timer -= dt;
            burn.tick_elapsed += dt;
            if burn.tick_elapsed >= Burning::TICK_RATE {
                burn.tick_elapsed -= Burning::TICK_RATE;
                health.current -= burn.damage_per_tick as i32;
            }
            if burn.timer <= 0.0 { commands.entity(e_ent).remove::<Burning>(); false }
            else { true }
        } else { false };

        let active_slow = if let Some(mut slow) = slowed {
            slow.timer -= dt;
            if slow.timer <= 0.0 { commands.entity(e_ent).remove::<Slowed>(); false }
            else { true }
        } else { false };

        // Color priority: DamageFlinch handled elsewhere, we only set when no flinch
        // (DamageFlinch removes itself from the query via Without filter when active,
        //  so enemies here are guaranteed not flinching — skip the Without filter and
        //  just let update_damage_flinch win by running after us in the schedule)
        if active_burn {
            let flash = (t_secs * 7.0).floor() as u32 % 2 == 0;
            sprite.color = if flash { Color::srgb(1.0, 0.45, 0.1) } else { Color::srgb(0.85, 0.65, 0.15) };
        } else if active_slow {
            sprite.color = Color::srgb(0.35, 0.65, 1.0);
        }
        // If neither, telegraph/flinch systems or normal_q in update_enemy_telegraph
        // will restore the sprite to white.
    }
}

// ── Player input ──────────────────────────────────────────────────────────────

pub fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Velocity, &mut FacingDirection), With<Player>>,
) {
    let Ok((mut vel, mut facing)) = player_q.get_single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }

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
    mut enemy_q: Query<(&Transform, &mut Velocity, Option<&Slowed>), (With<Enemy>, Without<Player>)>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let player_pos = pt.translation.truncate();

    for (et, mut vel, slowed) in &mut enemy_q {
        let ep = et.translation.truncate();
        let dist = ep.distance(player_pos);
        let speed_mul = slowed.map(|s| s.factor).unwrap_or(1.0);

        if dist > ENEMY_CHASE_RADIUS || dist < ATTACK_RANGE * 0.75 {
            vel.linvel = Vec2::ZERO;
        } else {
            vel.linvel = (player_pos - ep).normalize_or_zero() * ENEMY_SPEED * speed_mul;
        }
    }
}

// ── Combat ────────────────────────────────────────────────────────────────────

pub fn combat_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    game_font: Res<GameFont>,
    mut player_q: Query<
        (Entity, &Transform, &mut Health, &Attack, &Defense, &mut AttackCooldown, &FacingDirection),
        With<Player>,
    >,
    mut enemy_q: Query<
        (Entity, &Transform, &mut Health, &Attack, &Defense, &mut AttackCooldown, Option<&Boss>, Option<&mut WindUp>),
        (With<Enemy>, Without<Player>),
    >,
    mut score: ResMut<GameScore>,
    mut game_map: ResMut<GameMap>,
    mut player_stats: ResMut<PlayerStats>,
) {
    let Ok((player_entity, pt, mut p_hp, p_atk, p_def, mut p_cd, facing)) =
        player_q.get_single_mut()
    else {
        return;
    };
    let player_pos = pt.translation.truncate();
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();

    let pressed = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyZ);
    let can_attack = pressed && p_cd.0 <= 0.0;

    if can_attack {
        let origin = player_pos + facing.0 * TILE_SIZE * 1.4;
        commands.spawn((
            Text2d::new(CH_SWORD),
            TextFont { font: game_font.0.clone(), font_size: TILE_SIZE * 1.25, ..default() },
            TextColor(Color::srgb(1.0, 1.0, 0.55)),
            Transform::from_xyz(origin.x, origin.y, 3.0),
            SwingEffect { elapsed: 0.0 },
            LevelEntity,
        ));
        p_cd.0 = ATTACK_COOLDOWN_SECS;
    }

    for (e_ent, et, mut e_hp, e_atk, e_def, mut e_cd, boss, wind_up) in &mut enemy_q {
        let dist = player_pos.distance(et.translation.truncate());
        let has_windup = wind_up.is_some();

        if dist < ATTACK_RANGE {
            // ── Player strikes enemy ───────────────────────────────────────
            if can_attack {
                let dmg = (p_atk.0 - e_def.0 + rng.gen_range(0..=3)).max(1);
                e_hp.current -= dmg;
                commands.entity(e_ent).insert(DamageFlinch {
                    timer: 0.0,
                    normal_color: COL_NORMAL,
                    flash_color: Color::WHITE,
                });
            }

            // ── Enemy attack (two-phase telegraph) ────────────────────────
            if e_cd.0 <= 0.0 {
                if let Some(mut wu) = wind_up {
                    // Wind-up in progress — tick it down
                    wu.timer -= dt;
                    if wu.timer <= 0.0 {
                        // Strike!
                        let raw = (e_atk.0 - p_def.0 + rng.gen_range(0..=2)).max(1);
                        let dmg = if player_stats.is_blocking {
                            (raw as f32 * SHIELD_BLOCK_RATIO).round().max(1.0) as i32
                        } else {
                            raw
                        };
                        p_hp.current -= dmg;
                        e_cd.0 = ATTACK_COOLDOWN_SECS + 0.25;
                        commands.entity(e_ent).remove::<WindUp>();
                        let normal = if player_stats.shield_broken { COL_BROKEN }
                                     else if player_stats.is_blocking { COL_BLOCKING }
                                     else { COL_NORMAL };
                        commands.entity(player_entity).insert(DamageFlinch {
                            timer: 0.0,
                            normal_color: normal,
                            flash_color: Color::srgb(1.0, 0.15, 0.15),
                        });
                    }
                } else {
                    // Begin wind-up — telegraph with "!" above enemy
                    commands.entity(e_ent).insert(WindUp { timer: WindUp::DURATION });
                    let warn_pos = et.translation + Vec3::new(0.0, TILE_SIZE, 4.0);
                    commands.spawn((
                        Text2d::new("!"),
                        TextFont { font: game_font.0.clone(), font_size: TILE_SIZE * 1.1, ..default() },
                        TextColor(Color::srgb(1.0, 0.75, 0.0)),
                        Transform::from_translation(warn_pos),
                        AttackWarning { target: e_ent },
                        LevelEntity,
                    ));
                }
            }
        } else if has_windup {
            // Player dodged out of range — cancel the wind-up
            commands.entity(e_ent).remove::<WindUp>();
            e_cd.0 = ATTACK_COOLDOWN_SECS * 0.5;
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

// ── Enemy telegraph visuals ───────────────────────────────────────────────────

/// Pulses winding-up enemies orange→red; restores idle enemies to normal colour.
pub fn update_enemy_telegraph(
    mut winding_q: Query<
        (&WindUp, &mut Sprite, Option<&Boss>),
        (With<Enemy>, Without<DamageFlinch>),
    >,
    mut normal_q: Query<
        &mut Sprite,
        (With<Enemy>, Without<WindUp>, Without<DamageFlinch>, Without<Burning>, Without<Slowed>),
    >,
) {
    for (wu, mut sprite, boss) in &mut winding_q {
        // Pulse period shrinks as the strike approaches (urgency ramps up)
        let progress = 1.0 - (wu.timer / WindUp::DURATION).clamp(0.0, 1.0);
        let period = 0.12 - progress * 0.08; // 120 ms → 40 ms
        let flash = (wu.timer / period.max(0.04)).floor() as u32 % 2 == 0;
        let (bright, dim) = if boss.is_some() {
            (Color::srgb(1.0, 0.45, 0.0), Color::srgb(0.8, 0.15, 0.0))
        } else {
            (Color::srgb(1.0, 0.8, 0.0), Color::srgb(1.0, 0.35, 0.0))
        };
        sprite.color = if flash { bright } else { dim };
    }
    for mut sprite in &mut normal_q {
        sprite.color = COL_NORMAL;
    }
}

/// Keeps the "!" warning positioned above its target enemy; removes it when
/// the wind-up ends (attack landed or dodged).
pub fn update_attack_warnings(
    mut commands: Commands,
    mut warnings: Query<(Entity, &AttackWarning, &mut Transform), Without<Enemy>>,
    enemies: Query<&Transform, (With<Enemy>, With<WindUp>)>,
) {
    for (warn_entity, warning, mut warn_transform) in &mut warnings {
        match enemies.get(warning.target) {
            Ok(et) => {
                warn_transform.translation = et.translation + Vec3::new(0.0, TILE_SIZE, 4.0);
            }
            Err(_) => {
                commands.entity(warn_entity).despawn_recursive();
            }
        }
    }
}

// ── Damage flinch / flicker ───────────────────────────────────────────────────

pub fn update_damage_flinch(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DamageFlinch, &mut Sprite)>,
) {
    for (entity, mut flinch, mut sprite) in &mut q {
        flinch.timer += time.delta_secs();
        if flinch.timer >= DamageFlinch::DURATION {
            sprite.color = flinch.normal_color;
            commands.entity(entity).remove::<DamageFlinch>();
        } else {
            // Flicker at ~12 Hz
            let flash = (flinch.timer / 0.055).floor() as u32 % 2 == 0;
            sprite.color = if flash { flinch.flash_color } else { flinch.normal_color };
        }
    }
}

// ── Loot popups ───────────────────────────────────────────────────────────────

pub fn update_loot_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut LootPopup, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut popup, mut tf, mut color) in &mut q {
        popup.elapsed += dt;
        let t = (popup.elapsed / LootPopup::DURATION).clamp(0.0, 1.0);
        if t >= 1.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }
        tf.translation.y += TILE_SIZE * 1.6 * dt;
        let alpha = if t > 0.55 { 1.0 - (t - 0.55) / 0.45 } else { 1.0 };
        *color = TextColor(Color::srgba(popup.r, popup.g, popup.b, alpha));
    }
}

// ── Swing effect animation ────────────────────────────────────────────────────

pub fn update_swing_effects(
    mut commands: Commands,
    time: Res<Time>,
    player_q: Query<(&Transform, &FacingDirection), With<Player>>,
    mut q: Query<
        (Entity, &mut SwingEffect, &mut Transform, &mut TextColor),
        (With<SwingEffect>, Without<Player>),
    >,
) {
    let (player_pos, facing_dir) = player_q
        .get_single()
        .map(|(t, f)| (t.translation.truncate(), f.0))
        .unwrap_or((Vec2::ZERO, Vec2::X));

    // Angle the facing direction points in world space
    let facing_angle = facing_dir.y.atan2(facing_dir.x);

    for (entity, mut effect, mut transform, mut color) in &mut q {
        effect.elapsed += time.delta_secs();
        let t = (effect.elapsed / SwingEffect::DURATION).clamp(0.0, 1.0);

        if t >= 1.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        // Sweep offset goes +1 → -1 over the duration
        let sweep = 1.0 - t * 2.0;

        // Rotate the glyph: sweep ±80° around the facing direction.
        // † points UP by default, so facing_angle - PI/2 aligns it along facing_dir.
        let angle = (facing_angle - std::f32::consts::FRAC_PI_2) + sweep * std::f32::consts::PI * 0.45;
        transform.rotation = Quat::from_rotation_z(angle);

        // Anchor the hilt at the player: blade direction is the glyph's local +Y rotated by angle.
        let blade_dir = Vec2::new(-angle.sin(), angle.cos());
        let half_len = TILE_SIZE * 1.1;
        let pos = player_pos + blade_dir * half_len;
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        // Fade out in the final 35%
        let alpha = if t > 0.65 { 1.0 - (t - 0.65) / 0.35 } else { 1.0 };
        *color = TextColor(Color::srgba(1.0, 1.0, 0.55, alpha));
    }
}

// ── Chest / Item pickup ───────────────────────────────────────────────────────

pub fn check_item_pickup(
    mut commands: Commands,
    game_font: Res<GameFont>,
    player_q: Query<&Transform, With<Player>>,
    chest_q: Query<(Entity, &Transform), With<Chest>>,
    item_q: Query<(Entity, &Transform, &Item)>,
    mut player_stats: ResMut<PlayerStats>,
    mut score: ResMut<GameScore>,
    mut player_health_q: Query<&mut Health, With<Player>>,
    mut loot_log: ResMut<LootLog>,
    mut player_spells: ResMut<PlayerSpells>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let pp = pt.translation.truncate();
    let mut rng = rand::thread_rng();

    for (ce, ct) in &chest_q {
        if pp.distance(ct.translation.truncate()) < TILE_SIZE * 0.9 {
            let msg = apply_random_loot(&mut player_stats, &mut score, &mut rng, &mut player_health_q, &mut player_spells);
            loot_log.push(msg.clone());
            spawn_loot_popup(&mut commands, &game_font, ct.translation.truncate(), &msg);
            commands.entity(ce).despawn_recursive();
        }
    }

    for (ie, it, item) in &item_q {
        if pp.distance(it.translation.truncate()) < TILE_SIZE * 0.9 {
            let msg = apply_item(&item.0, &mut player_stats, &mut score, &mut player_health_q, &mut player_spells);
            loot_log.push(msg.clone());
            spawn_loot_popup(&mut commands, &game_font, it.translation.truncate(), &msg);
            commands.entity(ie).despawn_recursive();
        }
    }
}

fn apply_random_loot(
    stats: &mut PlayerStats,
    score: &mut GameScore,
    rng: &mut impl Rng,
    health_q: &mut Query<&mut Health, With<Player>>,
    spells: &mut PlayerSpells,
) -> String {
    // Collect spells the player is still missing
    let all = [SpellType::IceShard, SpellType::Lightning, SpellType::PoisonCloud];
    let missing: Vec<SpellType> = all.iter().copied()
        .filter(|s| !spells.known.contains(s))
        .collect();

    // If spells are available, add them as a 5th outcome (~20% chance)
    let pool = if missing.is_empty() { 4u32 } else { 5 };

    match rng.gen_range(0..pool) {
        0 => { stats.attack += 1;  score.score += 5; "ATK +1".to_string() }
        1 => { stats.defense += 1; score.score += 5; "DEF +1".to_string() }
        2 => {
            let heal = rng.gen_range(6..=12i32);
            heal_player(stats, health_q, heal);
            score.score += 5;
            format!("HP +{}", heal)
        }
        3 => {
            let gold = rng.gen_range(5u32..=25);
            score.score += gold;
            format!("+{} GOLD", gold)
        }
        _ => {
            let spell = missing[rng.gen_range(0..missing.len())];
            spells.known.push(spell);
            score.score += 20;
            format!("Scroll: {}", spell.name())
        }
    }
}

fn apply_item(
    item: &ItemType,
    stats: &mut PlayerStats,
    score: &mut GameScore,
    health_q: &mut Query<&mut Health, With<Player>>,
    spells: &mut PlayerSpells,
) -> String {
    match item {
        ItemType::Weapon  => { stats.attack += 1;  score.score += 5; "ATK +1".to_string() }
        ItemType::Armor   => { stats.defense += 1; score.score += 5; "DEF +1".to_string() }
        ItemType::Potion  => { heal_player(stats, health_q, 10); score.score += 5; "HP +10".to_string() }
        ItemType::Coins(n) => { score.score += n; format!("+{} GOLD", n) }
        ItemType::SpellScroll(spell) => {
            if !spells.known.contains(spell) { spells.known.push(*spell); }
            score.score += 20;
            format!("Scroll: {}", spell.name())
        }
    }
}

fn spawn_loot_popup(commands: &mut Commands, font: &GameFont, world_pos: Vec2, text: &str) {
    let (r, g, b) = loot_rgb(text);
    commands.spawn((
        Text2d::new(text),
        TextFont { font: font.0.clone(), font_size: TILE_SIZE * 0.95, ..default() },
        TextColor(Color::srgb(r, g, b)),
        Transform::from_xyz(world_pos.x, world_pos.y + TILE_SIZE, 5.0),
        LootPopup { elapsed: 0.0, r, g, b },
        LevelEntity,
    ));
}

fn loot_rgb(msg: &str) -> (f32, f32, f32) {
    if msg.starts_with("ATK")        { (1.0,  0.50, 0.15) } // orange
    else if msg.starts_with("DEF")   { (0.30, 0.80, 1.0)  } // cyan
    else if msg.starts_with("HP")    { (0.25, 1.0,  0.45) } // green
    else if msg.starts_with("Scroll"){ (0.75, 0.35, 1.0)  } // purple
    else                             { (1.0,  0.88, 0.15) } // gold
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
    let Ok(pt) = player_q.get_single() else { return; };
    let pp = pt.translation.truncate();

    for (lt, mut vis) in &mut ladder_q {
        if game_map.boss_dead { *vis = Visibility::Visible; }
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
    let Ok(hp) = player_q.get_single() else { return; };
    player_stats.hp = hp.current;
    if hp.current <= 0 { next_state.set(GameState::GameOver); }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  RENDERING / FOG OF WAR
// ═══════════════════════════════════════════════════════════════════════════════

pub fn update_fog_of_war(
    player_q: Query<&Transform, With<Player>>,
    mut game_map: ResMut<GameMap>,
) {
    let Ok(pt) = player_q.get_single() else { return; };
    let (px, py) = world_to_grid(pt.translation.truncate());

    for f in game_map.fog.iter_mut() {
        if *f == FogState::Visible { *f = FogState::Explored; }
    }

    for dy in -VIEW_RADIUS..=VIEW_RADIUS {
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            let tx = px + dx;
            let ty = py + dy;
            if tx < 0 || ty < 0 || tx >= MAP_WIDTH as i32 || ty >= MAP_HEIGHT as i32 { continue; }
            if dx * dx + dy * dy > VIEW_RADIUS * VIEW_RADIUS { continue; }
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
        if x == x1 && y == y1 { return true; }
        if (x != x0 || y != y0) && map.tile_at(x, y) == TileType::Wall { return false; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x += sx; }
        if e2 < dx  { err += dx; y += sy; }
    }
}

pub fn update_tile_rendering(
    game_map: Res<GameMap>,
    game_textures: Res<GameTextures>,
    mut tile_q: Query<(&TilePos, &mut Sprite, &mut Visibility)>,
) {
    for (tp, mut sprite, mut vis) in &mut tile_q {
        let idx = game_map.idx(tp.x, tp.y);
        match game_map.fog[idx] {
            FogState::Hidden => {
                *vis = Visibility::Hidden;
            }
            FogState::Explored => {
                *vis = Visibility::Visible;
                sprite.image = if game_map.tiles[idx] == TileType::Wall {
                    game_textures.wall_dimmed.clone()
                } else {
                    game_textures.floor_dimmed.clone()
                };
                sprite.color = Color::WHITE;
            }
            FogState::Visible => {
                *vis = Visibility::Visible;
                sprite.image = if game_map.tiles[idx] == TileType::Wall {
                    game_textures.wall.clone()
                } else {
                    game_textures.floor.clone()
                };
                sprite.color = Color::WHITE;
            }
        }
    }
}

pub fn update_entity_visibility(
    game_map: Res<GameMap>,
    mut q: Query<
        (&Transform, &mut Visibility),
        (With<LevelEntity>, Without<TilePos>, Without<Player>, Without<Node>, Without<LootPopup>),
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

fn spawn_hud(commands: &mut Commands, font: Handle<Font>, level: u32, hp: i32, max_hp: i32, stamina: f32, max_stamina: f32, loot_log: &LootLog) {
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
            // Top bar
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
                    TextFont { font: font.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                    HudLevelText,
                ));
                row.spawn((
                    Node::default(),
                    Text::new("Score: 0   Kills: 0"),
                    TextFont { font: font.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.82, 0.1)),
                    HudScoreText,
                ));
            });

            // Loot log panel — absolute, right-side below the score
            root.spawn(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(36.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(1.0),
                ..default()
            })
            .with_children(|col| {
                col.spawn((
                    Node::default(),
                    Text::new("LOOT"),
                    TextFont { font: font.clone(), font_size: 11.0, ..default() },
                    TextColor(Color::srgb(0.55, 0.55, 0.35)),
                ));
                for i in 0..LootLog::MAX_ENTRIES {
                    let entry = loot_log.entries.get(i).map(String::as_str).unwrap_or("");
                    let (r, g, b) = loot_rgb(entry);
                    col.spawn((
                        Node::default(),
                        Text::new(entry),
                        TextFont { font: font.clone(), font_size: 13.0, ..default() },
                        TextColor(Color::srgba(r, g, b, 0.85)),
                        HudLootLogText(i),
                        LevelEntity,
                    ));
                }
            });

            // Bottom bar
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
                    Text::new(format!(
                        "HP {}/{}  ATK {}  DEF {}   SP {}/{}",
                        hp, max_hp, 5, 2,
                        stamina as i32, max_stamina as i32
                    )),
                    TextFont { font: font.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.15, 1.0, 0.45)),
                    HudHealthText,
                ));
                row.spawn((
                    Node::default(),
                    Text::new("[Q] Fireball  MP: 60/60  |  E: cycle"),
                    TextFont { font: font.clone(), font_size: 14.0, ..default() },
                    TextColor(Color::srgb(0.55, 0.55, 1.0)),
                    HudSpellText,
                ));
                row.spawn((
                    Node::default(),
                    Text::new("WASD: Move  Space/Z: Atk  Shift: Block  Q: Spell  E: Cycle"),
                    TextFont { font: font.clone(), font_size: 11.0, ..default() },
                    TextColor(Color::srgb(0.4, 0.4, 0.45)),
                ));
            });
        });
}

pub fn update_hud(
    score: Res<GameScore>,
    player_stats: Res<PlayerStats>,
    current_level: Res<CurrentLevel>,
    loot_log: Res<LootLog>,
    player_spells: Res<PlayerSpells>,
    mut hp_q: Query<
        &mut Text,
        (With<HudHealthText>, Without<HudScoreText>, Without<HudLevelText>, Without<HudLootLogText>, Without<HudSpellText>),
    >,
    mut score_q: Query<
        &mut Text,
        (With<HudScoreText>, Without<HudHealthText>, Without<HudLevelText>, Without<HudLootLogText>, Without<HudSpellText>),
    >,
    mut level_q: Query<
        &mut Text,
        (With<HudLevelText>, Without<HudHealthText>, Without<HudScoreText>, Without<HudLootLogText>, Without<HudSpellText>),
    >,
    mut log_q: Query<
        (&HudLootLogText, &mut Text),
        (Without<HudHealthText>, Without<HudScoreText>, Without<HudLevelText>, Without<HudSpellText>),
    >,
    mut spell_q: Query<
        &mut Text,
        (With<HudSpellText>, Without<HudHealthText>, Without<HudScoreText>, Without<HudLevelText>, Without<HudLootLogText>),
    >,
) {
    for mut t in &mut hp_q {
        let shield_label = if player_stats.shield_broken {
            "  [SHIELD BROKEN]"
        } else if player_stats.is_blocking {
            "  [BLOCKING]"
        } else {
            ""
        };
        *t = Text::new(format!(
            "HP {}/{}  ATK {}  DEF {}   SP {}/{}{}",
            player_stats.hp, player_stats.max_hp,
            player_stats.attack, player_stats.defense,
            player_stats.stamina as i32, player_stats.max_stamina as i32,
            shield_label
        ));
    }
    for mut t in &mut score_q {
        *t = Text::new(format!("Score: {}   Kills: {}", score.score, score.kills));
    }
    for mut t in &mut level_q {
        *t = Text::new(format!("Floor  {}/{}", current_level.0, NUM_LEVELS));
    }
    for (slot, mut t) in &mut log_q {
        let entry = loot_log.entries.get(slot.0).map(String::as_str).unwrap_or("");
        *t = Text::new(entry);
    }
    for mut t in &mut spell_q {
        let spell = player_spells.known.get(player_spells.active).copied();
        let name = spell.map(|s| s.name()).unwrap_or("—");
        let cd = if player_spells.cooldown > 0.0 {
            format!(" (cd {:.1}s)", player_spells.cooldown)
        } else {
            String::new()
        };
        *t = Text::new(format!(
            "[Q] {}{}  |  MP {}/{}  |  E: cycle",
            name, cd,
            player_stats.mana as i32,
            player_stats.max_mana as i32,
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MENUS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn setup_main_menu(mut commands: Commands, game_font: Res<GameFont>) {
    let font = game_font.0.clone();
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.04, 0.06)),
            MenuEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Node::default(),
                Text::new("DESCENT"),
                TextFont { font: font.clone(), font_size: 70.0, ..default() },
                TextColor(Color::srgb(1.0, 0.8, 0.05)),
            ));
            p.spawn((
                Node::default(),
                Text::new("Descend five deadly floors and slay the final boss"),
                TextFont { font: font.clone(), font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
            p.spawn(Node { height: Val::Px(18.0), ..default() });
            p.spawn((
                Node::default(),
                Text::new("[ PRESS  ENTER  TO  START ]"),
                TextFont { font: font.clone(), font_size: 30.0, ..default() },
                TextColor(Color::srgb(0.25, 1.0, 0.5)),
            ));
            p.spawn(Node { height: Val::Px(24.0), ..default() });

            for &line in &[
                "WASD / Arrow Keys  --  Move",
                "Space / Z  --  Melee attack",
                "Q  --  Cast active spell  (costs mana)",
                "E  --  Cycle to next spell",
                "Hold Shift  --  Block  (reduces damage to 15%, drains stamina)",
                "  Stamina runs out → Shield breaks!  Wait 2s to recover.",
                "Walk over a chest  --  Open it  (weapon / armor / potion / coins / scrolls)",
                "Kill the BOSS (☠)  --  Ladder (↓) appears",
                "Use the ladder  --  Descend to the next floor",
                "",
                "  @  Player       ☻  Enemy       ☠  Boss",
                "  ▣  Chest        ↓  Ladder",
            ] {
                p.spawn((
                    Node::default(),
                    Text::new(line),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
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
    mut loot_log: ResMut<LootLog>,
    mut player_spells: ResMut<PlayerSpells>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        *player_stats = PlayerStats::default();
        *score = GameScore::default();
        *current_level = CurrentLevel(1);
        *loot_log = LootLog::default();
        *player_spells = PlayerSpells::default();
        next_state.set(GameState::Playing);
    }
}

pub fn setup_game_over(
    mut commands: Commands,
    score: Res<GameScore>,
    current_level: Res<CurrentLevel>,
    game_font: Res<GameFont>,
) {
    let font = game_font.0.clone();
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
            p.spawn((Node::default(), Text::new("YOU  DIED"),
                TextFont { font: font.clone(), font_size: 80.0, ..default() },
                TextColor(Color::srgb(0.9, 0.07, 0.07))));
            p.spawn((Node::default(), Text::new(format!("Reached floor  {}", current_level.0)),
                TextFont { font: font.clone(), font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.72, 0.72, 0.72))));
            p.spawn((Node::default(), Text::new(format!("Final Score:  {}", score.score)),
                TextFont { font: font.clone(), font_size: 42.0, ..default() },
                TextColor(Color::srgb(1.0, 0.82, 0.1))));
            p.spawn((Node::default(), Text::new(format!("Enemies killed:  {}", score.kills)),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6))));
            p.spawn(Node { height: Val::Px(20.0), ..default() });
            p.spawn((Node::default(), Text::new("[ ENTER -- Return to Menu ]"),
                TextFont { font: font.clone(), font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.4, 0.75, 1.0))));
        });
}

pub fn setup_victory(
    mut commands: Commands,
    score: Res<GameScore>,
    game_font: Res<GameFont>,
) {
    let font = game_font.0.clone();
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
            p.spawn((Node::default(), Text::new("VICTORY!"),
                TextFont { font: font.clone(), font_size: 80.0, ..default() },
                TextColor(Color::srgb(0.15, 1.0, 0.35))));
            p.spawn((Node::default(), Text::new("You have conquered the dungeon!"),
                TextFont { font: font.clone(), font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.9))));
            p.spawn((Node::default(), Text::new(format!("Final Score:  {}", score.score)),
                TextFont { font: font.clone(), font_size: 44.0, ..default() },
                TextColor(Color::srgb(1.0, 0.82, 0.1))));
            p.spawn((Node::default(), Text::new(format!("Enemies killed:  {}", score.kills)),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6))));
            p.spawn(Node { height: Val::Px(20.0), ..default() });
            p.spawn((Node::default(), Text::new("[ ENTER -- Play Again ]"),
                TextFont { font: font.clone(), font_size: 26.0, ..default() },
                TextColor(Color::srgb(0.4, 0.75, 1.0))));
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
