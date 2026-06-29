use crate::inventory::Slot;
use crate::menu::MenuScreen;
use crate::meshing::{ChunkMesh, VoxelWorldResource};
use crate::player::PlayerState;
use bevy::prelude::*;
use std::sync::Mutex;

/// Touch / mobile input written by the WASM HTML overlay (see `index.html`).
#[derive(Resource, Default, Clone, Copy)]
pub struct MobileInput {
    pub is_mobile: bool,
    pub active: bool,
    pub move_vec: Vec2,
    pub jump: bool,
    pub jump_pressed: bool,
    pub sprint: bool,
    pub look_delta: Vec2,
    pub break_pressed: bool,
    pub place_pressed: bool,
    pub inventory_pressed: bool,
    pub pause_pressed: bool,
    pub resume_pressed: bool,
    pub restart_pressed: bool,
    pub inventory_slot_pressed: Option<usize>,
    pub hotbar_select: Option<usize>,
}

#[derive(Default)]
struct PendingMobileInput {
    is_mobile: bool,
    active: bool,
    move_vec: Vec2,
    jump: bool,
    jump_was: bool,
    sprint: bool,
    look_delta: Vec2,
    break_pressed: bool,
    place_pressed: bool,
    inventory_pressed: bool,
    pause_pressed: bool,
    resume_pressed: bool,
    restart_pressed: bool,
    inventory_slot_pressed: Option<usize>,
    hotbar_select: Option<usize>,
}

static PENDING: Mutex<PendingMobileInput> = Mutex::new(PendingMobileInput {
    is_mobile: false,
    active: false,
    move_vec: Vec2::new(0.0, 0.0),
    jump: false,
    jump_was: false,
    sprint: false,
    look_delta: Vec2::new(0.0, 0.0),
    break_pressed: false,
    place_pressed: false,
    inventory_pressed: false,
    pause_pressed: false,
    resume_pressed: false,
    restart_pressed: false,
    inventory_slot_pressed: None,
    hotbar_select: None,
});

fn with_pending<F>(f: F)
where
    F: FnOnce(&mut PendingMobileInput),
{
    if let Ok(mut pending) = PENDING.lock() {
        f(&mut pending);
    }
}

pub fn is_controlling(player: &PlayerState, mobile: &MobileInput, ui: &crate::menu::GameUiState) -> bool {
    if !crate::menu::is_playing(ui) {
        return false;
    }
    player.cursor_locked || (mobile.is_mobile && mobile.active)
}

#[cfg(target_arch = "wasm32")]
pub fn detect_mobile_device() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };

    if let Ok(search) = window.location().search() {
        if search.contains("mobile") {
            return true;
        }
    }

    let navigator = window.navigator();
    if let Ok(ua) = navigator.user_agent() {
        let mobile_ua = [
            "Android", "iPhone", "iPad", "iPod", "Mobile", "webOS", "BlackBerry", "IEMobile",
            "Opera Mini",
        ]
        .iter()
        .any(|needle| ua.contains(needle));
        if mobile_ua {
            return true;
        }
    }

    let touch_capable = navigator.max_touch_points() > 0;
    let narrow = window
        .inner_width()
        .ok()
        .and_then(|w| w.as_f64())
        .is_some_and(|w| w < 900.0);

    let coarse = window
        .match_media("(pointer: coarse)")
        .ok()
        .flatten()
        .is_some_and(|mq| mq.matches());

    coarse || (touch_capable && narrow)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn detect_mobile_device() -> bool {
    false
}

pub fn init_mobile(mut mobile: ResMut<MobileInput>) {
    mobile.is_mobile = detect_mobile_device();
    with_pending(|pending| pending.is_mobile = mobile.is_mobile);
}

pub fn sync_mobile_input(mut mobile: ResMut<MobileInput>) {
    if let Ok(pending) = PENDING.lock() {
        mobile.is_mobile = pending.is_mobile;
        mobile.active = pending.active;
        mobile.move_vec = pending.move_vec;
        mobile.jump = pending.jump;
        mobile.jump_pressed = pending.jump && !pending.jump_was;
        mobile.sprint = pending.sprint;
        mobile.look_delta += pending.look_delta;
        mobile.break_pressed |= pending.break_pressed;
        mobile.place_pressed |= pending.place_pressed;
        mobile.inventory_pressed |= pending.inventory_pressed;
        mobile.pause_pressed |= pending.pause_pressed;
        mobile.resume_pressed |= pending.resume_pressed;
        mobile.restart_pressed |= pending.restart_pressed;
        if let Some(index) = pending.inventory_slot_pressed {
            mobile.inventory_slot_pressed = Some(index);
        }
        if let Some(index) = pending.hotbar_select {
            mobile.hotbar_select = Some(index);
        }
    }

    with_pending(|pending| {
        pending.jump_was = pending.jump;
        pending.look_delta = Vec2::ZERO;
        pending.break_pressed = false;
        pending.place_pressed = false;
        pending.inventory_pressed = false;
        pending.pause_pressed = false;
        pending.resume_pressed = false;
        pending.restart_pressed = false;
        pending.inventory_slot_pressed = None;
        pending.hotbar_select = None;
    });
}

pub fn clear_mobile_frame(mut mobile: ResMut<MobileInput>) {
    mobile.look_delta = Vec2::ZERO;
    mobile.break_pressed = false;
    mobile.place_pressed = false;
    mobile.inventory_pressed = false;
    mobile.pause_pressed = false;
    mobile.resume_pressed = false;
    mobile.restart_pressed = false;
    mobile.inventory_slot_pressed = None;
    mobile.hotbar_select = None;
}

#[cfg(target_arch = "wasm32")]
fn set_body_class(class: &str, add: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };
    let class_list = body.class_list();
    if add {
        let _ = class_list.add_1(class);
    } else {
        let _ = class_list.remove_1(class);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn notify_mobile_ui_ready(
    world: Res<VoxelWorldResource>,
    meshes: Query<&ChunkMesh>,
    mut notified: Local<bool>,
    mut wait_frames: Local<u32>,
) {
    if *notified || !detect_mobile_device() {
        return;
    }
    *wait_frames += 1;
    let (pcx, pcz) = world.player_chunk;
    let near_meshed = meshes.iter().any(|mesh| {
        (mesh.chunk_x - pcx).abs() <= 1 && (mesh.chunk_z - pcz).abs() <= 1
    });
    if !near_meshed && *wait_frames < 360 {
        return;
    }
    *notified = true;
    set_body_class("mobile", true);
    set_body_class("ready", true);
    crate::wasm_entry::dismiss_loading_screen();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn notify_mobile_ui_ready(
    _world: Res<VoxelWorldResource>,
    _meshes: Query<&ChunkMesh>,
    _notified: Local<bool>,
    _wait_frames: Local<u32>,
) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_mobile_menu_class(_ui: Res<crate::menu::GameUiState>, _mobile: Res<MobileInput>) {}

#[cfg(target_arch = "wasm32")]
pub fn sync_mobile_menu_class(ui: Res<crate::menu::GameUiState>, mobile: Res<MobileInput>) {
    if !mobile.is_mobile {
        return;
    }
    set_body_class("menu-pause", ui.screen == MenuScreen::Pause);
    set_body_class("menu-inventory", ui.screen == MenuScreen::Inventory);
    set_body_class("playing", mobile.active && ui.screen == MenuScreen::Playing);
}

#[cfg(target_arch = "wasm32")]
mod game_ui_dom {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = ncUpdateGameUi)]
        pub fn nc_update_game_ui(
            screen: u32,
            hotbar_index: u32,
            picked_slot: i32,
            slots_csv: &str,
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub fn push_html_ui_state(
    screen: MenuScreen,
    hotbar_index: usize,
    picked_slot: Option<usize>,
    slots: &[Slot],
) {
    let screen_code = match screen {
        MenuScreen::Playing => 0,
        MenuScreen::Pause => 1,
        MenuScreen::Inventory => 2,
    };
    let mut csv = String::with_capacity(slots.len() * 8);
    for (i, slot) in slots.iter().enumerate() {
        if i > 0 {
            csv.push(';');
        }
        csv.push_str(&format!("{}:{}", slot.item, slot.count));
    }
    game_ui_dom::nc_update_game_ui(
        screen_code,
        hotbar_index as u32,
        picked_slot.map(|s| s as i32).unwrap_or(-1),
        &csv,
    );
}

#[cfg(target_arch = "wasm32")]
pub fn sync_mobile_hotbar_ui(
    inventory: Res<crate::inventory::GameInventory>,
    ui: Res<crate::menu::GameUiState>,
    mobile: Res<MobileInput>,
    mut last_signature: Local<u64>,
) {
    if !mobile.is_mobile {
        return;
    }
    let mut signature = inventory.hotbar_index as u64;
    for (i, slot) in inventory.slots.iter().take(9).enumerate() {
        signature = signature
            .wrapping_add(slot.item as u64 * ((i + 1) as u64))
            .wrapping_add(slot.count as u64);
    }
    if *last_signature == signature {
        return;
    }
    *last_signature = signature;
    push_html_ui_state(ui.screen, inventory.hotbar_index, ui.picked_slot, &inventory.slots);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_mobile_hotbar_ui(
    _inventory: Res<crate::inventory::GameInventory>,
    _ui: Res<crate::menu::GameUiState>,
    _mobile: Res<MobileInput>,
    _last_signature: Local<u64>,
) {}

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::{with_pending, PendingMobileInput};
    use bevy::prelude::Vec2;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn nc_set_mobile(is_mobile: bool) {
        with_pending(|pending: &mut PendingMobileInput| pending.is_mobile = is_mobile);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_start() {
        with_pending(|pending| pending.active = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_set_move(x: f32, z: f32) {
        with_pending(|pending| pending.move_vec = Vec2::new(x, z));
    }

    #[wasm_bindgen]
    pub fn nc_mobile_set_jump(pressed: bool) {
        with_pending(|pending| pending.jump = pressed);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_set_sprint(pressed: bool) {
        with_pending(|pending| pending.sprint = pressed);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_add_look(dx: f32, dy: f32) {
        with_pending(|pending| pending.look_delta += Vec2::new(dx, dy));
    }

    #[wasm_bindgen]
    pub fn nc_mobile_break() {
        with_pending(|pending| pending.break_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_place() {
        with_pending(|pending| pending.place_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_toggle_inventory() {
        with_pending(|pending| pending.inventory_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_open_pause() {
        with_pending(|pending| pending.pause_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_resume() {
        with_pending(|pending| pending.resume_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_mobile_restart() {
        with_pending(|pending| pending.restart_pressed = true);
    }

    #[wasm_bindgen]
    pub fn nc_inventory_click_slot(index: u32) {
        with_pending(|pending| pending.inventory_slot_pressed = Some(index as usize));
    }

    #[wasm_bindgen]
    pub fn nc_mobile_select_hotbar(index: u32) {
        with_pending(|pending| pending.hotbar_select = Some(index as usize));
    }
}
