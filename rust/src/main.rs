mod blocks;
mod chunk_gen;
mod config;
mod inventory;
mod meshing;
mod noise;
mod player;
mod structures;
mod ui;
mod weather;
mod world;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use config::DEFAULT_SEED;
use meshing::{sync_chunk_meshes, update_world_chunks, ChunkMaterial, RemeshQueue, VoxelWorldResource};
use player::{
    block_interaction, hotbar_keys, lock_cursor, mouse_look, player_movement, spawn_player,
    sync_camera, toggle_inventory, PlayerCamera, PlayerState,
};
use ui::{draw_hud, setup_fog, update_fps, HudState};
use weather::{update_day_night, update_lights};
use inventory::GameInventory;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Nodecraft — Rust Edition".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(VoxelWorldResource::new(DEFAULT_SEED))
        .insert_resource(PlayerState::default())
        .insert_resource(GameInventory::with_starter_items())
        .insert_resource(RemeshQueue::default())
        .insert_resource(HudState::default())
        .insert_resource(weather::DayNight::default())
        .add_systems(Startup, (setup_scene, setup_fog, spawn_player, init_world))
        .add_systems(
            Update,
            (
                lock_cursor,
                mouse_look,
                player_movement,
                sync_camera,
                block_interaction,
                hotbar_keys,
                toggle_inventory,
            ),
        )
        .add_systems(
            Update,
            (
                update_world_chunks,
                sync_chunk_meshes,
                update_day_night,
                update_lights,
                update_fps,
                draw_hud,
            ),
        )
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 600.0,
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.insert_resource(ChunkMaterial(mat));
}

fn init_world(mut world: ResMut<VoxelWorldResource>, mut queue: ResMut<RemeshQueue>) {
    world.loaded_chunks = world.inner.load_chunks_around(0, 0);
    crate::chunk_gen::ensure_settlements_near(&mut world.inner, 0, 0, 320);
    for &(cx, cz) in &world.loaded_chunks.clone() {
        queue.keys.push((cx, cz));
    }
}
