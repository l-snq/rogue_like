use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

mod components;
mod level_gen;
mod resources;
mod systems;

use resources::*;
use systems::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    /// One-frame bridge state so re-entering Playing always triggers OnExit/OnEnter.
    LevelTransition,
    GameOver,
    Victory,
}

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rogue Adventure".to_string(),
                resolution: (1280.0, 720.0).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }),
    )
    // Black background
    .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
    // Physics — all actors carry GravityScale(0.0)
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
    // One-time setup
    .add_systems(Startup, setup_camera)
    // State
    .init_state::<GameState>()
    // Resources
    .init_resource::<GameMap>()
    .init_resource::<GameScore>()
    .init_resource::<CurrentLevel>()
    .init_resource::<PlayerStats>()
    // ── Main Menu ──────────────────────────────────────────────────────────────
    .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
    .add_systems(OnExit(GameState::MainMenu), cleanup_entities::<components::MenuEntity>)
    .add_systems(Update, menu_input.run_if(in_state(GameState::MainMenu)))
    // ── Playing ────────────────────────────────────────────────────────────────
    .add_systems(OnEnter(GameState::Playing), setup_level)
    .add_systems(OnExit(GameState::Playing), cleanup_entities::<components::LevelEntity>)
    .add_systems(
        Update,
        (
            tick_cooldowns,
            player_input,
            enemy_ai,
            combat_system,
            update_damage_flinch,
            update_swing_effects,
            check_item_pickup,
            check_ladder,
            check_death,
            update_fog_of_war,
            update_tile_rendering,
            update_entity_visibility,
            update_hud,
        )
            .run_if(in_state(GameState::Playing)),
    )
    // ── Level Transition ───────────────────────────────────────────────────────
    .add_systems(OnEnter(GameState::LevelTransition), transition_level)
    // ── Game Over ──────────────────────────────────────────────────────────────
    .add_systems(OnEnter(GameState::GameOver), setup_game_over)
    .add_systems(OnExit(GameState::GameOver), cleanup_entities::<components::MenuEntity>)
    .add_systems(Update, end_screen_input.run_if(in_state(GameState::GameOver)))
    // ── Victory ────────────────────────────────────────────────────────────────
    .add_systems(OnEnter(GameState::Victory), setup_victory)
    .add_systems(OnExit(GameState::Victory), cleanup_entities::<components::MenuEntity>)
    .add_systems(Update, end_screen_input.run_if(in_state(GameState::Victory)));

    // Insert the font resource directly into the world BEFORE app.run() so it is
    // guaranteed to exist when OnEnter(MainMenu) fires (which happens in the first
    // PreUpdate, before any Startup-system commands would be flushed).
    let font_handle = app
        .world()
        .resource::<AssetServer>()
        .load("fonts/JetBrainsMonoNerdFont-Regular.ttf");
    app.world_mut().insert_resource(GameFont(font_handle));

    app.run();
}
