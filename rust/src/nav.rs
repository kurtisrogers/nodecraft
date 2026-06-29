use crate::config::SEA_LEVEL;
use crate::menu::{is_playing, GameUiState};
use crate::meshing::VoxelWorldResource;
use crate::noise::NoiseGenerator;
use crate::player::PlayerState;
use crate::world::VoxelWorld;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy_egui::{egui, EguiContexts};

pub const SETTLEMENT_GRID: i32 = 192;
pub const VOLCANO_GRID: i32 = 384;
pub const MINIMAP_SIZE: usize = 56;
pub const MINIMAP_RADIUS: i32 = 56;
const LANDMARK_SEARCH_RADIUS: i32 = 1024;
const MINIMAP_REFRESH_SECS: f32 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandmarkKind {
    Village,
    Volcano,
}

impl LandmarkKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Village => "Village",
            Self::Volcano => "Volcano",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Landmark {
    pub kind: LandmarkKind,
    pub x: i32,
    pub z: i32,
    pub distance: f32,
    pub bearing: f32,
    pub discovered: bool,
}

#[derive(Resource)]
pub struct NavHud {
    pub minimap_rgba: Vec<u8>,
    pub minimap_dirty: bool,
    pub landmarks: Vec<Landmark>,
    pub nearest_village: Option<Landmark>,
    pub nearest_volcano: Option<Landmark>,
    pub refresh_timer: f32,
}

impl Default for NavHud {
    fn default() -> Self {
        Self {
            minimap_rgba: vec![0; MINIMAP_SIZE * MINIMAP_SIZE * 4],
            minimap_dirty: true,
            landmarks: Vec::new(),
            nearest_village: None,
            nearest_volcano: None,
            refresh_timer: 0.0,
        }
    }
}

pub fn update_nav_hud(
    time: Res<Time>,
    ui: Res<GameUiState>,
    player: Res<PlayerState>,
    world: Res<VoxelWorldResource>,
    mut nav: ResMut<NavHud>,
) {
    if !is_playing(&ui) {
        return;
    }

    nav.refresh_timer -= time.delta_secs();
    if nav.refresh_timer > 0.0 {
        return;
    }
    nav.refresh_timer = MINIMAP_REFRESH_SECS;

    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;
    nav.landmarks = collect_landmarks(&world.inner, px, pz, LANDMARK_SEARCH_RADIUS);
    nav.nearest_village = nav
        .landmarks
        .iter()
        .copied()
        .filter(|l| l.kind == LandmarkKind::Village)
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    nav.nearest_volcano = nav
        .landmarks
        .iter()
        .copied()
        .filter(|l| l.kind == LandmarkKind::Volcano)
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));

    let landmarks = nav.landmarks.clone();
    build_minimap(
        &mut nav.minimap_rgba,
        &world.inner.noise,
        &landmarks,
        px,
        pz,
        player.yaw,
    );
    nav.minimap_dirty = true;

    #[cfg(target_arch = "wasm32")]
    push_nav_html(&nav, player.yaw);
}

pub fn collect_landmarks(world: &VoxelWorld, px: i32, pz: i32, radius: i32) -> Vec<Landmark> {
    let mut landmarks = Vec::new();
    let noise = &world.noise;

    let min_cell_x = (px - radius).div_euclid(SETTLEMENT_GRID);
    let max_cell_x = (px + radius).div_euclid(SETTLEMENT_GRID);
    let min_cell_z = (pz - radius).div_euclid(SETTLEMENT_GRID);
    let max_cell_z = (pz + radius).div_euclid(SETTLEMENT_GRID);

    for cell_x in min_cell_x..=max_cell_x {
        for cell_z in min_cell_z..=max_cell_z {
            if !settlement_cell_valid(noise, cell_x, cell_z) {
                continue;
            }
            let center_x = cell_x * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
            let center_z = cell_z * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
            let dist = block_distance(px, pz, center_x, center_z);
            if dist > radius as f32 {
                continue;
            }
            let discovered = world.placed_settlements.contains(&(cell_x, cell_z));
            landmarks.push(Landmark {
                kind: LandmarkKind::Village,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
                discovered,
            });
        }
    }

    let vmin_x = (px - radius).div_euclid(VOLCANO_GRID);
    let vmax_x = (px + radius).div_euclid(VOLCANO_GRID);
    let vmin_z = (pz - radius).div_euclid(VOLCANO_GRID);
    let vmax_z = (pz + radius).div_euclid(VOLCANO_GRID);

    for cell_x in vmin_x..=vmax_x {
        for cell_z in vmin_z..=vmax_z {
            if noise.volcano_cell_score(cell_x, cell_z) < 0.24 {
                continue;
            }
            let center_x = cell_x * VOLCANO_GRID + VOLCANO_GRID / 2;
            let center_z = cell_z * VOLCANO_GRID + VOLCANO_GRID / 2;
            if !noise.is_land(center_x, center_z) {
                continue;
            }
            if noise.peaks_layer(center_x, center_z) < 0.08 {
                continue;
            }
            if noise.settlement_at(center_x, center_z) > 0.2 {
                continue;
            }
            let dist = block_distance(px, pz, center_x, center_z);
            if dist > radius as f32 {
                continue;
            }
            let discovered = world.placed_volcanoes.contains(&(cell_x, cell_z));
            landmarks.push(Landmark {
                kind: LandmarkKind::Volcano,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
                discovered,
            });
        }
    }

    landmarks.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    landmarks
}

fn settlement_cell_valid(noise: &NoiseGenerator, cell_x: i32, cell_z: i32) -> bool {
    let cell = noise.fbm(
        cell_x as f32 * 0.85 + 500.0,
        cell_z as f32 * 0.85 + 500.0,
        2,
        0.5,
        2.0,
    );
    if cell < 0.18 {
        return false;
    }
    let center_x = cell_x * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
    let center_z = cell_z * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
    noise.is_land(center_x, center_z)
        && noise.terrain_height(center_x, center_z) > SEA_LEVEL + 2
        && noise.local_flatness(center_x, center_z) >= 0.7
}

fn block_distance(ax: i32, az: i32, bx: i32, bz: i32) -> f32 {
    let dx = (bx - ax) as f32;
    let dz = (bz - az) as f32;
    (dx * dx + dz * dz).sqrt()
}

/// Bearing in world space: 0 = north (−Z), increasing clockwise.
pub fn world_bearing(from_x: i32, from_z: i32, to_x: i32, to_z: i32) -> f32 {
    let dx = (to_x - from_x) as f32;
    let dz = (to_z - from_z) as f32;
    dx.atan2(-dz)
}

pub fn relative_bearing(player_yaw: f32, landmark_bearing: f32) -> f32 {
    wrap_angle(landmark_bearing - player_yaw)
}

fn wrap_angle(a: f32) -> f32 {
    let mut v = a % std::f32::consts::TAU;
    if v > std::f32::consts::PI {
        v -= std::f32::consts::TAU;
    }
    if v < -std::f32::consts::PI {
        v += std::f32::consts::TAU;
    }
    v
}

pub fn build_minimap(
    out: &mut [u8],
    noise: &NoiseGenerator,
    landmarks: &[Landmark],
    player_x: i32,
    player_z: i32,
    player_yaw: f32,
) {
    let size = MINIMAP_SIZE as i32;
    let half = MINIMAP_RADIUS;

    for py in 0..size {
        for px in 0..size {
            let wx = player_x + px - half;
            let wz = player_z + py - half;
            let idx = ((py * size + px) * 4) as usize;
            let rgba = terrain_color(noise, wx, wz);
            out[idx..idx + 4].copy_from_slice(&rgba);
        }
    }

    for landmark in landmarks {
        let mx = landmark.x - player_x + half;
        let mz = landmark.z - player_z + half;
        let color = match landmark.kind {
            LandmarkKind::Village if landmark.discovered => [255, 210, 80, 255],
            LandmarkKind::Village => [255, 180, 60, 180],
            LandmarkKind::Volcano if landmark.discovered => [255, 90, 60, 255],
            LandmarkKind::Volcano => [220, 70, 50, 170],
        };
        stamp_marker(out, size, mx, mz, 2, color);
    }

    stamp_player(out, size, half, half, player_yaw);
}

fn terrain_color(noise: &NoiseGenerator, wx: i32, wz: i32) -> [u8; 4] {
    if !noise.is_land(wx, wz) || noise.terrain_height(wx, wz) <= SEA_LEVEL {
        return [36, 78, 140, 255];
    }
    let biome = noise.biome(wx, wz);
    if biome.temperature < -0.3 {
        return [214, 224, 236, 255];
    }
    if biome.temperature > 0.3 && biome.moisture < -0.1 {
        return [196, 176, 96, 255];
    }
    if noise.settlement_at(wx, wz) > 0.35 {
        return [156, 132, 84, 255];
    }
    if noise.volcano_at(wx, wz) > 0.25 {
        return [108, 72, 64, 255];
    }
    [52, 118, 48, 255]
}

fn stamp_marker(out: &mut [u8], size: i32, cx: i32, cz: i32, radius: i32, color: [u8; 4]) {
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            let x = cx + dx;
            let z = cz + dz;
            if x < 0 || z < 0 || x >= size || z >= size {
                continue;
            }
            let idx = ((z * size + x) * 4) as usize;
            out[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn stamp_player(out: &mut [u8], size: i32, cx: i32, cz: i32, yaw: f32) {
    stamp_marker(out, size, cx, cz, 1, [250, 250, 255, 255]);
    let forward = Vec2::new(-yaw.sin(), -yaw.cos());
    let tip_x = cx + (forward.x * 4.0).round() as i32;
    let tip_z = cz + (forward.y * 4.0).round() as i32;
    stamp_marker(out, size, tip_x, tip_z, 0, [250, 250, 255, 255]);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn draw_nav_hud(
    mut contexts: EguiContexts,
    ui_state: Res<GameUiState>,
    player: Res<PlayerState>,
    mut nav: ResMut<NavHud>,
    mobile: Res<crate::mobile::MobileInput>,
    mut texture_cache: Local<Option<egui::TextureHandle>>,
) {
    if mobile.is_mobile || ui_state.screen != crate::menu::MenuScreen::Playing {
        return;
    }

    let ctx = contexts.ctx_mut();
    if nav.minimap_dirty || texture_cache.is_none() {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [MINIMAP_SIZE, MINIMAP_SIZE],
            &nav.minimap_rgba,
        );
        *texture_cache = Some(ctx.load_texture(
            "minimap",
            image,
            egui::TextureOptions::NEAREST,
        ));
        nav.minimap_dirty = false;
    }
    draw_nav_panel(ctx, &nav, player.yaw, texture_cache.as_ref());
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_nav_panel(
    ctx: &egui::Context,
    nav: &NavHud,
    player_yaw: f32,
    texture_cache: Option<&egui::TextureHandle>,
) {
    egui::Area::new(egui::Id::new("nav_hud"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                draw_wayfinder(ui, nav, player_yaw);
                ui.add_space(6.0);
                if let Some(tex) = texture_cache {
                    let size = MINIMAP_SIZE as f32;
                    ui.image((tex.id(), egui::vec2(size, size)));
                }
                ui.label(
                    egui::RichText::new("N")
                        .small()
                        .color(egui::Color32::from_rgb(200, 200, 210)),
                );
            });
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_wayfinder(ui: &mut egui::Ui, nav: &NavHud, player_yaw: f32) {
    let panel_w = MINIMAP_SIZE as f32;
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(panel_w, panel_w * 0.42),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = rect.width() * 0.38;

    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(220, 220, 230, 200)),
    );
    painter.circle_filled(
        center,
        2.0,
        egui::Color32::from_rgb(240, 240, 250),
    );

    if let Some(village) = nav.nearest_village {
        let rel = relative_bearing(player_yaw, village.bearing);
        let arrow = egui::vec2(rel.sin(), -rel.cos()) * radius * 0.82;
        let tip = center + arrow;
        let wing = egui::vec2(-arrow.y, arrow.x).normalized() * 5.0;
        painter.line_segment(
            [center + wing * 0.35, tip],
            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 210, 80)),
        );
        painter.line_segment(
            [center - wing * 0.35, tip],
            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 210, 80)),
        );

        let status = if village.discovered { "nearby" } else { "ahead" };
        painter.text(
            rect.left_bottom() + egui::vec2(0.0, -2.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{} {:.0}m {}", village.kind.label(), village.distance, status),
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(235, 235, 245),
        );
    } else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No village\nin range",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(180, 180, 190),
        );
    }

    if let Some(volcano) = nav.nearest_volcano {
        let rel = relative_bearing(player_yaw, volcano.bearing);
        let dot = center + egui::vec2(rel.sin(), -rel.cos()) * radius * 0.62;
        painter.circle_filled(dot, 3.5, egui::Color32::from_rgb(255, 100, 70));
        painter.text(
            rect.right_bottom() + egui::vec2(0.0, -2.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{} {:.0}m", volcano.kind.label(), volcano.distance),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(255, 150, 120),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn push_nav_html(nav: &NavHud, player_yaw: f32) {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = ncUpdateNavHud)]
        fn nc_update_nav_hud(
            player_yaw: f32,
            village_dist: f32,
            village_bearing: f32,
            village_active: bool,
            volcano_dist: f32,
            volcano_bearing: f32,
            volcano_active: bool,
            rgba: &[u8],
        );
    }

    let (v_dist, v_bearing, v_active) = nav
        .nearest_village
        .map(|v| (v.distance, relative_bearing(player_yaw, v.bearing), true))
        .unwrap_or((0.0, 0.0, false));
    let (vol_dist, vol_bearing, vol_active) = nav
        .nearest_volcano
        .map(|v| (v.distance, relative_bearing(player_yaw, v.bearing), true))
        .unwrap_or((0.0, 0.0, false));

    nc_update_nav_hud(
        player_yaw,
        v_dist,
        v_bearing,
        v_active,
        vol_dist,
        vol_bearing,
        vol_active,
        &nav.minimap_rgba,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::VoxelWorld;

    #[test]
    fn minimap_buffer_matches_size() {
        let world = VoxelWorld::new(42);
        let mut buf = vec![0u8; MINIMAP_SIZE * MINIMAP_SIZE * 4];
        build_minimap(&mut buf, &world.noise, &[], 0, 0, 0.0);
        assert_eq!(buf.len(), MINIMAP_SIZE * MINIMAP_SIZE * 4);
    }

    #[test]
    fn finds_settlement_landmark_near_origin() {
        let world = VoxelWorld::new(42);
        let landmarks = collect_landmarks(&world, 0, 0, 512);
        assert!(
            landmarks.iter().any(|l| l.kind == LandmarkKind::Village),
            "expected at least one village landmark near origin"
        );
    }

    #[test]
    fn relative_bearing_is_zero_when_facing_target() {
        let bearing = world_bearing(0, 0, 0, -20);
        let rel = relative_bearing(bearing, bearing);
        assert!(rel.abs() < 0.001);
    }
}
