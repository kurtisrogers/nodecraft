mod blocks;
mod chunk_gen;
mod config;
mod inventory;
mod meshing;
mod mobile;
mod mobs;
mod noise;
mod player;
mod structures;
mod ui;
mod weather;
mod world;

use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use config::DEFAULT_SEED;
use meshing::{sync_chunk_meshes, update_world_chunks, ChunkMaterial, RemeshQueue, VoxelWorldResource};
use mobile::{clear_mobile_frame, init_mobile, notify_mobile_ui_ready, sync_mobile_input, sync_mobile_menu_class, MobileInput};
use mobs::{mob_attack_interaction, mob_ai, mob_spawner, MobManager};
use player::{
    block_interaction, hotbar_keys, lock_cursor, mouse_look, player_movement, spawn_player,
    sync_camera, toggle_inventory, PlayerCamera, PlayerState,
};
use ui::{draw_hud, setup_fog, update_fps, HudState};
use weather::{update_day_night, update_lights};
use inventory::GameInventory;

pub fn run() {
    let wasm = cfg!(target_arch = "wasm32");
    let title = if wasm {
        "Nodecraft — Rust WASM"
    } else {
        "Nodecraft — Rust Edition"
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: title.into(),
                    present_mode: if wasm {
                        PresentMode::AutoNoVsync
                    } else {
                        PresentMode::AutoVsync
                    },
                    canvas: if wasm {
                        Some("#canvas".into())
                    } else {
                        None
                    },
                    fit_canvas_to_parent: wasm,
                    prevent_default_event_handling: wasm,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::asset::AssetPlugin {
                watch_for_changes_override: if wasm { Some(false) } else { None },
                ..default()
            }),
    )
    .add_plugins(EguiPlugin)
    .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
    .insert_resource(VoxelWorldResource::new(DEFAULT_SEED))
    .insert_resource(PlayerState::default())
    .insert_resource(GameInventory::with_starter_items())
    .insert_resource(RemeshQueue::default())
    .insert_resource(HudState::default())
    .insert_resource(MobileInput::default())
    .insert_resource(weather::DayNight::default())
    .insert_resource(mobs::MobManager::default())
    .add_systems(Startup, (setup_scene, setup_fog, spawn_player, init_world, init_mobile))
    .add_systems(
        Update,
        (
            sync_mobile_input,
            lock_cursor,
            mouse_look,
            player_movement,
            sync_camera,
            mob_attack_interaction,
            block_interaction,
            hotbar_keys,
            toggle_inventory,
            clear_mobile_frame,
            sync_mobile_menu_class,
            notify_mobile_ui_ready,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            mob_spawner,
            mob_ai,
            update_world_chunks,
            sync_chunk_meshes,
            update_day_night,
            update_lights,
            update_fps,
            draw_hud,
        ),
    );

    app.run();
}

fn setup_scene(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.85, 0.88, 0.92),
        brightness: 400.0,
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: !cfg!(target_arch = "wasm32"),
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    commands.insert_resource(ChunkMaterial(mat));
}

fn init_world(
    mut world: ResMut<VoxelWorldResource>,
    mut queue: ResMut<RemeshQueue>,
    player: Res<PlayerState>,
) {
    if cfg!(target_arch = "wasm32") {
        world.inner.render_distance = 6;
    }
    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;
    world.player_chunk = (
        px.div_euclid(crate::config::CHUNK_SIZE),
        pz.div_euclid(crate::config::CHUNK_SIZE),
    );
    world.loaded_chunks = world.inner.load_chunks_around(px, pz);
    crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, 320);
    for &(cx, cz) in &world.loaded_chunks.clone() {
        queue.keys.push((cx, cz));
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_entry {
    use wasm_bindgen::prelude::*;

    fn hide_loading_overlay() {
        use wasm_bindgen::JsCast;
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(element) = document.get_element_by_id("loading") else {
            return;
        };
        if let Ok(html_element) = element.dyn_into::<web_sys::HtmlElement>() {
            let _ = html_element.class_list().add_1("hidden");
        }
    }

    #[wasm_bindgen(start)]
    pub fn wasm_start() {
        console_error_panic_hook::set_once();
        hide_loading_overlay();
        // Defer Bevy so Trunk can finish init and expose window.wasmBindings.
        wasm_bindgen_futures::spawn_local(async {
            super::run();
        });
    }
}
