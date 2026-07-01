use crate::config::{DEFAULT_SEED, HOTBAR_SIZE, INVENTORY_SIZE};
use crate::inventory::GameInventory;
use crate::meshing::{ChunkEntityMap, ChunkMesh, RemeshQueue, VoxelWorldResource};
use crate::mobs::MobEntity;
use crate::mobile::MobileInput;
use crate::player::PlayerState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuScreen {
    #[default]
    Playing,
    Pause,
    Inventory,
}

#[derive(Resource, Default)]
pub struct GameUiState {
    pub screen: MenuScreen,
    pub picked_slot: Option<usize>,
    pub restart_requested: bool,
    pub html_dirty: bool,
}

pub fn is_playing(ui: &GameUiState) -> bool {
    ui.screen == MenuScreen::Playing
}

pub fn handle_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<GameUiState>,
    mut inventory: ResMut<GameInventory>,
    mobile: Res<MobileInput>,
) {
    if mobile.restart_pressed {
        ui.restart_requested = true;
    }
    if mobile.resume_pressed {
        close_to_playing(&mut ui);
    }
    if mobile.pause_pressed {
        toggle_pause(&mut ui);
    } else if mobile.inventory_pressed {
        match ui.screen {
            MenuScreen::Inventory => close_to_playing(&mut ui),
            MenuScreen::Pause => open_inventory(&mut ui),
            MenuScreen::Playing => open_inventory(&mut ui),
        }
    } else if let Some(slot) = mobile.inventory_slot_pressed {
        click_slot(&mut ui, &mut inventory, slot);
    }

    if keys.just_pressed(KeyCode::Escape) {
        match ui.screen {
            MenuScreen::Inventory => {
                ui.screen = MenuScreen::Pause;
                ui.picked_slot = None;
            }
            MenuScreen::Pause => close_to_playing(&mut ui),
            MenuScreen::Playing => {
                ui.screen = MenuScreen::Pause;
                ui.picked_slot = None;
            }
        }
        ui.html_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        match ui.screen {
            MenuScreen::Pause => open_inventory(&mut ui),
            MenuScreen::Inventory => close_to_playing(&mut ui),
            MenuScreen::Playing => open_inventory(&mut ui),
        }
        ui.html_dirty = true;
    }
}

fn toggle_pause(ui: &mut GameUiState) {
    ui.screen = match ui.screen {
        MenuScreen::Pause => MenuScreen::Playing,
        _ => MenuScreen::Pause,
    };
    ui.picked_slot = None;
    ui.html_dirty = true;
}

fn open_inventory(ui: &mut GameUiState) {
    ui.screen = MenuScreen::Inventory;
    ui.picked_slot = None;
    ui.html_dirty = true;
}

fn close_to_playing(ui: &mut GameUiState) {
    ui.screen = MenuScreen::Playing;
    ui.picked_slot = None;
    ui.html_dirty = true;
}

pub fn click_slot(ui: &mut GameUiState, inventory: &mut GameInventory, slot: usize) {
    if slot >= INVENTORY_SIZE {
        return;
    }
    ui.html_dirty = true;

    if slot < HOTBAR_SIZE && ui.screen == MenuScreen::Inventory {
        inventory.hotbar_index = slot;
        ui.picked_slot = None;
        return;
    }

    match ui.picked_slot {
        None => {
            if inventory.slots[slot].item != 0 {
                ui.picked_slot = Some(slot);
            }
        }
        Some(first) if first == slot => {
            ui.picked_slot = None;
        }
        Some(first) => {
            inventory.swap_slots(first, slot);
            ui.picked_slot = None;
        }
    }
}

pub fn release_cursor_when_menu_open(
    ui: Res<GameUiState>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut player: ResMut<PlayerState>,
) {
    let Ok(mut window) = window.get_single_mut() else {
        return;
    };
    if ui.screen != MenuScreen::Playing {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        player.cursor_locked = false;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_restart(
    mut commands: Commands,
    mut ui: ResMut<GameUiState>,
    mut world: ResMut<VoxelWorldResource>,
    mut player: ResMut<PlayerState>,
    mut inventory: ResMut<GameInventory>,
    mut queue: ResMut<RemeshQueue>,
    mut entity_map: ResMut<ChunkEntityMap>,
    mobs: Query<Entity, With<MobEntity>>,
    chunk_meshes: Query<Entity, With<ChunkMesh>>,
) {
    if !ui.restart_requested {
        return;
    }
    ui.restart_requested = false;

    for entity in mobs.iter() {
        commands.entity(entity).despawn();
    }
    for entity in chunk_meshes.iter() {
        commands.entity(entity).despawn();
    }
    entity_map.entities.clear();
    queue.keys.clear();
    queue.clear_pending();

    *world = VoxelWorldResource::new(DEFAULT_SEED);
    *inventory = GameInventory::with_starter_items();
    *player = PlayerState::default();

    let spawn = world.inner.find_safe_spawn(0, 0);
    player.position = Vec3::new(spawn.0, spawn.1, spawn.2);
    player.velocity = Vec3::ZERO;
    player.pitch = if cfg!(target_arch = "wasm32") { -0.35 } else { 0.0 };
    crate::collision::ensure_clear(&world.inner, &mut player.position, crate::player::PLAYER_AABB);

    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;
    world.player_chunk = (
        px.div_euclid(crate::config::CHUNK_SIZE),
        pz.div_euclid(crate::config::CHUNK_SIZE),
    );

    world.loaded_chunks = world.inner.load_chunks_around(px, pz);
    let loaded_set: std::collections::HashSet<_> = world.loaded_chunks.iter().copied().collect();
    world.inner.retain_chunks(&loaded_set);
    crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, 320);
    for &(cx, cz) in &world.loaded_chunks.clone() {
        queue.push((cx, cz));
    }

    ui.screen = MenuScreen::Playing;
    ui.picked_slot = None;
    ui.html_dirty = true;
}

#[cfg(target_arch = "wasm32")]
pub fn process_restart(mut ui: ResMut<GameUiState>) {
    if !ui.restart_requested {
        return;
    }
    ui.restart_requested = false;
    if let Some(window) = web_sys::window() {
        let _ = window.location().reload();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn sync_html_ui(
    mut ui: ResMut<GameUiState>,
    inventory: Res<GameInventory>,
    mut last_signature: Local<u64>,
) {
    let mut signature: u64 = ui.screen as u64;
    signature = signature.wrapping_mul(31).wrapping_add(inventory.hotbar_index as u64);
    for (i, slot) in inventory.slots.iter().enumerate() {
        signature = signature
            .wrapping_add((slot.item as u64).wrapping_mul((i as u64 + 1) * 17))
            .wrapping_add(slot.count as u64);
    }
    signature = signature
        .wrapping_mul(31)
        .wrapping_add(ui.picked_slot.map(|s| s as u64 + 1).unwrap_or(0));

    if *last_signature == signature {
        return;
    }
    *last_signature = signature;
    ui.html_dirty = false;

    crate::mobile::push_html_ui_state(ui.screen, inventory.hotbar_index, ui.picked_slot, &inventory.slots);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_html_ui(
    _ui: Res<GameUiState>,
    _inventory: Res<GameInventory>,
    _last_signature: Local<u64>,
) {
}
