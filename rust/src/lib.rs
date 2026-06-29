mod assets;
mod chunk_material;
mod blocks;
mod chunk_gen;
mod collision;
mod config;
mod decorations;
mod menu;
mod inventory;
mod meshing;
mod mobile;
mod mobs;
mod noise;
mod player;
mod clouds;
mod sky;
mod structures;
mod ui;
mod weather;
mod world;
mod world_gen;
mod fp_view;

use bevy::prelude::*;
use bevy::window::PresentMode;
#[cfg(not(target_arch = "wasm32"))]
use bevy_egui::EguiPlugin;
use chunk_material::{VoxelChunkMaterial, VoxelChunkMaterialPlugin};
use config::{DEFAULT_SEED, WASM_BOOT_CHUNK_RADIUS, WASM_RENDER_DISTANCE};
use meshing::{
    bootstrap_player_meshes, sync_chunk_meshes, update_world_chunks, ChunkEntityMap, ChunkMaterial,
    RemeshQueue, VoxelWorldResource,
};
use mobile::{
    clear_mobile_frame, init_mobile, notify_mobile_ui_ready, sync_mobile_hotbar_ui, sync_mobile_input,
    sync_mobile_menu_class, MobileInput,
};
use mobs::{mob_attack_interaction, mob_ai, mob_spawner};
use menu::{
    handle_menu_input, process_restart, release_cursor_when_menu_open, sync_html_ui, GameUiState,
};
use player::{
    block_interaction, hotbar_keys, lock_cursor, mobile_session_start,
    mouse_look, player_movement, spawn_player, sync_camera, update_terrain_ready,
    PlayerState,
};
use ui::{setup_fog, update_fps, HudState};
#[cfg(not(target_arch = "wasm32"))]
use ui::draw_hud;
use weather::{MoonLight, SunLight, update_day_night, update_lights};
use inventory::GameInventory;
use clouds::{setup_clouds, update_clouds};
use decorations::{billboard_foliage, setup_decorations};
use fp_view::{setup_fp_view, update_fp_view};
use sky::{setup_sky, update_sky};

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
    .add_plugins(VoxelChunkMaterialPlugin)
    .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)));
    if !wasm {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(EguiPlugin);
    }
    app
    .insert_resource(VoxelWorldResource::new(DEFAULT_SEED))
    .insert_resource(PlayerState::default())
    .insert_resource(GameInventory::with_starter_items())
    .insert_resource(RemeshQueue::default())
    .insert_resource(ChunkEntityMap::default())
    .insert_resource(HudState::default())
    .insert_resource(MobileInput::default())
    .insert_resource(GameUiState::default())
    .insert_resource(weather::DayNight::default())
    .insert_resource(mobs::MobManager::default())
    .add_systems(Startup, (
        setup_scene,
        setup_fog,
        setup_decorations,
        spawn_player,
        init_world,
        init_mobile,
    ))
    .add_systems(PostStartup, (bootstrap_player_meshes, setup_fp_view));
    app.add_systems(Startup, (setup_clouds, setup_sky));
    app
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
            handle_menu_input,
            release_cursor_when_menu_open,
            clear_mobile_frame,
            sync_mobile_menu_class,
            sync_mobile_hotbar_ui,
            mobile_session_start,
            notify_mobile_ui_ready,
            sync_html_ui,
            process_restart,
            update_fp_view,
        )
            .chain(),
    )
    .add_systems(Update, billboard_foliage)
    .add_systems(
        Update,
        (
            update_world_chunks,
            sync_chunk_meshes,
            update_terrain_ready,
            update_day_night,
            update_lights,
            update_fps,
        ),
    );
    app.add_systems(Update, update_clouds);
    app.add_systems(Update, update_sky.after(update_lights));
    app.add_systems(Update, (mob_spawner, mob_ai));
    if !wasm {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, draw_hud);
    }

    #[cfg(target_arch = "wasm32")]
    {
        app.add_systems(PostStartup, finish_wasm_startup);
    }

    app.run();
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm_entry {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wasm_bindgen::prelude::*;

    static CHUNK_MESH_COUNT: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn set_chunk_mesh_count(count: usize) {
        CHUNK_MESH_COUNT.store(count, Ordering::Relaxed);
    }

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

    pub(crate) fn dismiss_loading_screen() {
        hide_loading_overlay();
    }

    pub(crate) fn hide_loading_overlay_if_ready(chunk_count: usize) {
        if chunk_count > 0 {
            hide_loading_overlay();
        }
    }

    #[wasm_bindgen]
    pub fn nc_chunk_mesh_count() -> usize {
        CHUNK_MESH_COUNT.load(Ordering::Relaxed)
    }

    #[wasm_bindgen(start)]
    pub fn wasm_start() {
        console_error_panic_hook::set_once();
        wasm_bindgen_futures::spawn_local(async {
            super::run();
        });
    }
}

fn setup_scene(
    mut commands: Commands,
    mut materials: ResMut<Assets<VoxelChunkMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.85, 0.88, 0.92),
        brightness: if cfg!(target_arch = "wasm32") { 1200.0 } else { 400.0 },
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: !cfg!(target_arch = "wasm32"),
            ..default()
        },
        SunLight,
        Transform::from_xyz(50.0, 100.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 0.0,
            shadows_enabled: false,
            ..default()
        },
        MoonLight,
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
    ));

    let mat = materials.add(VoxelChunkMaterial {});
    commands.insert_resource(ChunkMaterial(mat));
}

fn init_world(
    mut world: ResMut<VoxelWorldResource>,
    mut queue: ResMut<RemeshQueue>,
    player: Res<PlayerState>,
) {
    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;
    world.player_chunk = (
        px.div_euclid(crate::config::CHUNK_SIZE),
        pz.div_euclid(crate::config::CHUNK_SIZE),
    );

    if cfg!(target_arch = "wasm32") {
        world.inner.render_distance = WASM_BOOT_CHUNK_RADIUS;
        world.loaded_chunks = world.inner.load_chunks_around(px, pz);
        world.inner.render_distance = WASM_RENDER_DISTANCE;
    } else {
        world.loaded_chunks = world.inner.load_chunks_around(px, pz);
    }

    let loaded_set: std::collections::HashSet<_> = world.loaded_chunks.iter().copied().collect();
    world.inner.retain_chunks(&loaded_set);
    crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, 320);
    if cfg!(not(target_arch = "wasm32")) {
        crate::chunk_gen::ensure_volcanoes_near(&mut world.inner, px, pz, 320);
    }
    for &(cx, cz) in &world.loaded_chunks.clone() {
        queue.push((cx, cz));
    }
}

#[cfg(target_arch = "wasm32")]
fn finish_wasm_startup(entity_map: Res<ChunkEntityMap>) {
    if !entity_map.entities.is_empty() {
        wasm_entry::dismiss_loading_screen();
    }
}
