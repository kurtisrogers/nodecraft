use crate::config::{HOTBAR_SIZE, INVENTORY_SIZE};
use crate::inventory::GameInventory;
use crate::menu::{click_slot, GameUiState, MenuScreen};
use crate::mobs::MobManager;
use crate::mobile::MobileInput;
use crate::player::PlayerState;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy_egui::{egui, EguiContexts};

#[derive(Resource, Default)]
pub struct HudState {
    pub fps: f32,
    pub frame_accum: f32,
    pub frame_count: u32,
}

pub fn update_fps(time: Res<Time>, mut hud: ResMut<HudState>) {
    hud.frame_accum += time.delta_secs();
    hud.frame_count += 1;
    if hud.frame_accum >= 1.0 {
        hud.fps = hud.frame_count as f32 / hud.frame_accum;
        hud.frame_accum = 0.0;
        hud.frame_count = 0;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn draw_hud(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<GameUiState>,
    hud: Res<HudState>,
    player: Res<PlayerState>,
    mut inventory: ResMut<GameInventory>,
    mobs: Res<MobManager>,
    day: Res<crate::weather::DayNight>,
    mobile: Res<MobileInput>,
) {
    if mobile.is_mobile {
        return;
    }
    let ctx = contexts.ctx_mut();
    let hud_color = egui::Color32::from_rgb(235, 235, 245);

    if ui_state.screen == MenuScreen::Playing {
        egui::Area::new(egui::Id::new("hud_top"))
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(ctx, |ui| {
                ui.colored_label(hud_color, format!("Nodecraft Rust {}", crate::config::BUILD_VERSION));
                ui.colored_label(hud_color, format!("{:.0} FPS", hud.fps));
                ui.colored_label(
                    hud_color,
                    format!(
                        "{:.0}, {:.0}, {:.0}",
                        player.position.x, player.position.y, player.position.z
                    ),
                );
                ui.colored_label(hud_color, format!("Health: {}/20", player.health));
                ui.colored_label(hud_color, format!("Mobs nearby: {}", mobs.count));
                let cycle =
                    (day.time % crate::config::DAY_LENGTH_SECS) / crate::config::DAY_LENGTH_SECS;
                ui.colored_label(hud_color, if cycle > 0.5 { "Night" } else { "Day" });
            });

        egui::Area::new(egui::Id::new("hotbar"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for i in 0..HOTBAR_SIZE {
                        let label = hotbar_slot_label(&inventory, i);
                        let selected = i == inventory.hotbar_index;
                        if selected {
                            ui.colored_label(egui::Color32::YELLOW, label);
                        } else {
                            ui.label(label);
                        }
                    }
                });
            });
    }

    let show_desktop_help =
        ui_state.screen == MenuScreen::Playing && !player.cursor_locked;
    if show_desktop_help {
        egui::Area::new(egui::Id::new("help"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.heading("Nodecraft — Rust Edition");
                ui.label("Click to play");
                ui.label("WASD move · Space jump · Shift sprint");
                ui.label("LMB break · RMB place · E inventory · Esc pause · 1-9 hotbar");
            });
    }

    if ui_state.screen == MenuScreen::Pause {
        egui::Window::new("Paused")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Game paused");
                ui.add_space(8.0);
                if ui.button("Resume").clicked() {
                    ui_state.screen = MenuScreen::Playing;
                    ui_state.picked_slot = None;
                }
                if ui.button("Inventory").clicked() {
                    ui_state.screen = MenuScreen::Inventory;
                    ui_state.picked_slot = None;
                }
                if ui.button("Restart").clicked() {
                    ui_state.restart_requested = true;
                }
            });
    }

    if ui_state.screen == MenuScreen::Inventory {
        egui::Window::new("Inventory")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Click a slot to pick up, click another to swap. Keys 1-9 select hotbar.");
                ui.add_space(6.0);
                egui::Grid::new("inv_hotbar").show(ui, |ui| {
                    for i in 0..HOTBAR_SIZE {
                        draw_inventory_slot(ui, &mut ui_state, &mut inventory, i, true);
                    }
                    ui.end_row();
                });
                ui.add_space(8.0);
                egui::Grid::new("inv_grid").show(ui, |ui| {
                    for i in HOTBAR_SIZE..INVENTORY_SIZE {
                        draw_inventory_slot(ui, &mut ui_state, &mut inventory, i, false);
                        if (i - HOTBAR_SIZE + 1) % 9 == 0 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    ui_state.screen = MenuScreen::Playing;
                    ui_state.picked_slot = None;
                }
            });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn hotbar_slot_label(inventory: &GameInventory, index: usize) -> String {
    let slot = &inventory.slots[index];
    if slot.item == 0 {
        format!("[{}]", index + 1)
    } else {
        format!(
            "[{}] {}",
            index + 1,
            inventory.slot_label(index)
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_inventory_slot(
    ui: &mut egui::Ui,
    ui_state: &mut GameUiState,
    inventory: &mut GameInventory,
    index: usize,
    is_hotbar: bool,
) {
    let label = if inventory.slots[index].item == 0 {
        if is_hotbar {
            format!("{}", index + 1)
        } else {
            "-".to_string()
        }
    } else {
        inventory.slot_label(index)
    };
    let picked = ui_state.picked_slot == Some(index);
    let hotbar = inventory.hotbar_index == index;
    let prefix = if picked { "> " } else if hotbar { "* " } else { "" };
    if ui.button(format!("{prefix}{label}")).clicked() {
        click_slot(ui_state, inventory, index);
    }
}

pub fn setup_fog() {
    // Distance fog is handled via ClearColor and render distance for now.
}
