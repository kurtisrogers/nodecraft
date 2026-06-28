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

pub fn is_controlling(player: &PlayerState, mobile: &MobileInput) -> bool {
    if player.inventory_open {
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
        pending.hotbar_select = None;
    });
}

pub fn clear_mobile_frame(mut mobile: ResMut<MobileInput>) {
    mobile.look_delta = Vec2::ZERO;
    mobile.break_pressed = false;
    mobile.place_pressed = false;
    mobile.inventory_pressed = false;
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
pub fn sync_mobile_menu_class(_player: Res<PlayerState>, _mobile: Res<MobileInput>) {}

#[cfg(target_arch = "wasm32")]
pub fn sync_mobile_menu_class(_player: Res<PlayerState>, mobile: Res<MobileInput>) {
    if !mobile.is_mobile {
        return;
    }
    // Never disable mobile HTML controls — egui inventory is desktop-only.
    set_body_class("menu-open", false);
    set_body_class("playing", mobile.active);
}

#[cfg(target_arch = "wasm32")]
fn update_dom_hotbar(index: usize) {
    nc_update_mobile_hotbar(index as u32);
}

#[cfg(target_arch = "wasm32")]
mod hotbar_dom {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = ncUpdateMobileHotbar)]
        pub fn nc_update_mobile_hotbar(index: u32);
    }
}

#[cfg(target_arch = "wasm32")]
use hotbar_dom::nc_update_mobile_hotbar;

#[cfg(target_arch = "wasm32")]
pub fn sync_mobile_hotbar_ui(
    inventory: Res<crate::inventory::GameInventory>,
    mobile: Res<MobileInput>,
    mut last: Local<Option<usize>>,
) {
    if !mobile.is_mobile {
        return;
    }
    let index = inventory.hotbar_index;
    if *last != Some(index) {
        *last = Some(index);
        update_dom_hotbar(index);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_mobile_hotbar_ui(
    _inventory: Res<crate::inventory::GameInventory>,
    _mobile: Res<MobileInput>,
    _last: Local<Option<usize>>,
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
    pub fn nc_mobile_select_hotbar(index: u32) {
        with_pending(|pending| pending.hotbar_select = Some(index as usize));
    }
}
