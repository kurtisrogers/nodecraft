use crate::config::BUILD_VERSION;
use crate::inventory::GameInventory;
use crate::mobs::MobManager;
use crate::mobile::MobileInput;
use crate::player::PlayerState;
use bevy::prelude::*;
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

pub fn draw_hud(
    mut contexts: EguiContexts,
    hud: Res<HudState>,
    player: Res<PlayerState>,
    inventory: Res<GameInventory>,
    mobs: Res<MobManager>,
    day: Res<crate::weather::DayNight>,
    mobile: Res<MobileInput>,
) {
    let ctx = contexts.ctx_mut();

    egui::Area::new(egui::Id::new("hud_top"))
        .fixed_pos(egui::pos2(10.0, 10.0))
        .show(ctx, |ui| {
            ui.label(format!("Nodecraft Rust {BUILD_VERSION}"));
            ui.label(format!("{:.0} FPS", hud.fps));
            ui.label(format!(
                "{:.0}, {:.0}, {:.0}",
                player.position.x, player.position.y, player.position.z
            ));
            ui.label(format!("Health: {}/20", player.health));
            ui.label(format!("Mobs nearby: {}", mobs.count));
            let cycle = (day.time % crate::config::DAY_LENGTH_SECS) / crate::config::DAY_LENGTH_SECS;
            ui.label(if cycle > 0.5 { "Night" } else { "Day" });
        });

    egui::Area::new(egui::Id::new("hotbar"))
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, if mobile.is_mobile { -72.0 } else { -12.0 }),
        )
        .show(ctx, |ui| {
            if mobile.is_mobile {
                return;
            }
            ui.horizontal(|ui| {
                for i in 0..9 {
                    let slot = &inventory.slots[i];
                    let selected = i == inventory.hotbar_index;
                    let label = if slot.item == 0 {
                        format!("[{}]", i + 1)
                    } else {
                        format!("[{}] x{}", i + 1, slot.count)
                    };
                    if selected {
                        ui.colored_label(egui::Color32::YELLOW, label);
                    } else {
                        ui.label(label);
                    }
                }
            });
        });

    let show_desktop_help = !mobile.is_mobile && !player.cursor_locked && !player.inventory_open;
    if show_desktop_help {
        egui::Area::new(egui::Id::new("help"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.heading("Nodecraft — Rust Edition");
                ui.label("Click to play");
                ui.label("WASD move · Space jump · Shift sprint");
                ui.label("LMB break · RMB place · E inventory · 1-9 hotbar");
            });
    }

    if player.inventory_open && !mobile.is_mobile {
        egui::Window::new("Inventory")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("inv").show(ui, |ui| {
                    for (i, slot) in inventory.slots.iter().enumerate() {
                        let label = if slot.item == 0 {
                            "-".to_string()
                        } else {
                            format!("{} x{}", slot.item, slot.count)
                        };
                        if ui.selectable_label(i == inventory.hotbar_index, label).clicked() {
                            // hotbar selection from inventory click could be added
                        }
                        if (i + 1) % 9 == 0 {
                            ui.end_row();
                        }
                    }
                });
            });
    }
}

pub fn setup_fog() {
    // Distance fog is handled via ClearColor and render distance for now.
}
