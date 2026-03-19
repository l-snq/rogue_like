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
    /// One-frame intermediate state used to cleanly swap levels without
    /// re-entering the same state (Bevy won't trigger OnExit/OnEnter for
    /// a same-state transition).
    LevelTransition,
    GameOver,
    Victory,
}

fn main() {
    App::new()
        .add_plugins(
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
        // Black background (tiles use transparent text for "hidden" state)
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        // Physics — all actors have GravityScale(0.0) so default gravity is harmless
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
        // ── Main Menu ──────────────────────────────────────────────────────────
        .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
        .add_systems(OnExit(GameState::MainMenu), cleanup_entities::<components::MenuEntity>)
        .add_systems(Update, menu_input.run_if(in_state(GameState::MainMenu)))
        // ── Playing ────────────────────────────────────────────────────────────
        .add_systems(OnEnter(GameState::Playing), setup_level)
        .add_systems(OnExit(GameState::Playing), cleanup_entities::<components::LevelEntity>)
        .add_systems(
            Update,
            (
                tick_cooldowns,
                player_input,
                enemy_ai,
                combat_system,
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
        // ── Level Transition ───────────────────────────────────────────────────
        .add_systems(OnEnter(GameState::LevelTransition), transition_level)
        // ── Game Over ──────────────────────────────────────────────────────────
        .add_systems(OnEnter(GameState::GameOver), setup_game_over)
        .add_systems(OnExit(GameState::GameOver), cleanup_entities::<components::MenuEntity>)
        .add_systems(Update, end_screen_input.run_if(in_state(GameState::GameOver)))
        // ── Victory ────────────────────────────────────────────────────────────
        .add_systems(OnEnter(GameState::Victory), setup_victory)
        .add_systems(OnExit(GameState::Victory), cleanup_entities::<components::MenuEntity>)
        .add_systems(Update, end_screen_input.run_if(in_state(GameState::Victory)))
        .run();
}
