use crate::blocks::BlockId;
use crate::config::{SEA_LEVEL, WORLD_HEIGHT};
use crate::menu::{is_playing, GameUiState};
use crate::meshing::VoxelWorldResource;
use crate::mobs::MobEntity;
use crate::noise::NoiseGenerator;
use crate::player::PlayerState;
use crate::world::VoxelWorld;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy_egui::{egui, EguiContexts};

pub const SETTLEMENT_GRID: i32 = 192;
pub const VOLCANO_GRID: i32 = 384;
pub const MINIMAP_SIZE: usize = 80;
pub const MINIMAP_RADIUS: i32 = 80;
/// On-screen size in pixels (desktop egui + mobile CSS).
pub const MINIMAP_DISPLAY_PX: f32 = 168.0;
const LANDMARK_SEARCH_RADIUS: i32 = 1024;
const MINIMAP_REFRESH_SECS: f32 = 0.12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandmarkKind {
    Village,
    Volcano,
}

#[derive(Clone, Copy, Debug)]
pub struct Landmark {
    pub kind: LandmarkKind,
    pub x: i32,
    pub z: i32,
    pub discovered: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct MobBlip {
    pub x: i32,
    pub z: i32,
    pub hostile: bool,
}

#[derive(Resource)]
pub struct NavHud {
    pub minimap_rgba: Vec<u8>,
    pub minimap_dirty: bool,
    pub landmarks: Vec<Landmark>,
    pub mob_blips: Vec<MobBlip>,
    pub refresh_timer: f32,
}

impl Default for NavHud {
    fn default() -> Self {
        Self {
            minimap_rgba: vec![0; MINIMAP_SIZE * MINIMAP_SIZE * 4],
            minimap_dirty: true,
            landmarks: Vec::new(),
            mob_blips: Vec::new(),
            refresh_timer: 0.0,
        }
    }
}

pub fn update_nav_hud(
    time: Res<Time>,
    ui: Res<GameUiState>,
    player: Res<PlayerState>,
    mut world: ResMut<VoxelWorldResource>,
    mobs: Query<(&MobEntity, &Transform)>,
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

    let preview = collect_landmarks(&world.inner, px, pz, LANDMARK_SEARCH_RADIUS);
    if let Some(village) = preview
        .iter()
        .find(|l| l.kind == LandmarkKind::Village && !l.discovered)
    {
        let dist = block_distance(px, pz, village.x, village.z);
        if dist < 400.0 {
            crate::chunk_gen::ensure_settlements_near(&mut world.inner, village.x, village.z, 128);
        }
    }

    nav.landmarks = collect_landmarks(&world.inner, px, pz, LANDMARK_SEARCH_RADIUS);
    nav.mob_blips = collect_mob_blips(&mobs);

    let landmarks = nav.landmarks.clone();
    let mob_blips = nav.mob_blips.clone();
    build_minimap(
        &mut nav.minimap_rgba,
        &world.inner,
        &landmarks,
        &mob_blips,
        px,
        pz,
        player.yaw,
    );
    nav.minimap_dirty = true;

    #[cfg(target_arch = "wasm32")]
    push_minimap_html(&nav.minimap_rgba);
}

fn collect_mob_blips(mobs: &Query<(&MobEntity, &Transform)>) -> Vec<MobBlip> {
    mobs.iter()
        .filter(|(mob, _)| mob.alive)
        .map(|(mob, transform)| MobBlip {
            x: transform.translation.x.floor() as i32,
            z: transform.translation.z.floor() as i32,
            hostile: mob.kind.is_hostile(),
        })
        .collect()
}

pub fn collect_landmarks(world: &VoxelWorld, px: i32, pz: i32, radius: i32) -> Vec<Landmark> {
    let mut landmarks = Vec::new();
    let noise = &world.noise;

    for &(center_x, center_z) in &world.settlement_centers {
        if block_distance(px, pz, center_x, center_z) <= radius as f32 {
            landmarks.push(Landmark {
                kind: LandmarkKind::Village,
                x: center_x,
                z: center_z,
                discovered: true,
            });
        }
    }

    for &(center_x, center_z) in &world.volcano_centers {
        if block_distance(px, pz, center_x, center_z) <= radius as f32 {
            landmarks.push(Landmark {
                kind: LandmarkKind::Volcano,
                x: center_x,
                z: center_z,
                discovered: true,
            });
        }
    }

    let min_cell_x = (px - radius).div_euclid(SETTLEMENT_GRID);
    let max_cell_x = (px + radius).div_euclid(SETTLEMENT_GRID);
    let min_cell_z = (pz - radius).div_euclid(SETTLEMENT_GRID);
    let max_cell_z = (pz + radius).div_euclid(SETTLEMENT_GRID);

    for cell_x in min_cell_x..=max_cell_x {
        for cell_z in min_cell_z..=max_cell_z {
            if world.placed_settlements.contains(&(cell_x, cell_z)) {
                continue;
            }
            if !settlement_cell_valid(noise, cell_x, cell_z) {
                continue;
            }
            let center_x = cell_x * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
            let center_z = cell_z * SETTLEMENT_GRID + SETTLEMENT_GRID / 2;
            if block_distance(px, pz, center_x, center_z) > radius as f32 {
                continue;
            }
            landmarks.push(Landmark {
                kind: LandmarkKind::Village,
                x: center_x,
                z: center_z,
                discovered: false,
            });
        }
    }

    let vmin_x = (px - radius).div_euclid(VOLCANO_GRID);
    let vmax_x = (px + radius).div_euclid(VOLCANO_GRID);
    let vmin_z = (pz - radius).div_euclid(VOLCANO_GRID);
    let vmax_z = (pz + radius).div_euclid(VOLCANO_GRID);

    for cell_x in vmin_x..=vmax_x {
        for cell_z in vmin_z..=vmax_z {
            if world.placed_volcanoes.contains(&(cell_x, cell_z)) {
                continue;
            }
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
            if block_distance(px, pz, center_x, center_z) > radius as f32 {
                continue;
            }
            landmarks.push(Landmark {
                kind: LandmarkKind::Volcano,
                x: center_x,
                z: center_z,
                discovered: false,
            });
        }
    }

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

/// Map world offset to screen pixel (heading-up: forward = top of map).
/// Matches player forward `(-sin(yaw), -cos(yaw))` and right `(cos(yaw), -sin(yaw))`.
fn world_offset_to_map(dx: f32, dz: f32, yaw: f32, half: i32) -> (i32, i32) {
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let right = dx * cos_yaw - dz * sin_yaw;
    let ahead = -dx * sin_yaw - dz * cos_yaw;
    (
        (half as f32 + right).round() as i32,
        (half as f32 - ahead).round() as i32,
    )
}

pub fn build_minimap(
    out: &mut [u8],
    world: &VoxelWorld,
    landmarks: &[Landmark],
    mob_blips: &[MobBlip],
    player_x: i32,
    player_z: i32,
    player_yaw: f32,
) {
    let size = MINIMAP_SIZE as i32;
    let half = MINIMAP_RADIUS;
    let cos_yaw = player_yaw.cos();
    let sin_yaw = player_yaw.sin();
    let radius_sq = (half as f32 - 1.5).powi(2);

    for pixel in out.chunks_mut(4) {
        pixel.copy_from_slice(&[10, 14, 22, 255]);
    }

    for py in 0..size {
        for px in 0..size {
            let right = (px - half) as f32;
            let ahead = (half - py) as f32;
            if right * right + ahead * ahead > radius_sq {
                continue;
            }
            let dx = right * cos_yaw - ahead * sin_yaw;
            let dz = right * sin_yaw - ahead * cos_yaw;
            let wx = player_x + dx.round() as i32;
            let wz = player_z + dz.round() as i32;
            let idx = ((py * size + px) * 4) as usize;
            out[idx..idx + 4].copy_from_slice(&terrain_color_world(world, wx, wz));
        }
    }

    draw_ring(out, size, half);

    for landmark in landmarks {
        let dx = (landmark.x - player_x) as f32;
        let dz = (landmark.z - player_z) as f32;
        let (mx, mz) = world_offset_to_map(dx, dz, player_yaw, half);
        let color = match landmark.kind {
            LandmarkKind::Village if landmark.discovered => [255, 220, 60, 255],
            LandmarkKind::Village => [255, 200, 60, 160],
            LandmarkKind::Volcano if landmark.discovered => [255, 80, 50, 255],
            LandmarkKind::Volcano => [220, 60, 40, 160],
        };
        let radius = if landmark.kind == LandmarkKind::Village { 4 } else { 3 };
        stamp_marker(out, size, mx, mz, radius, color);
    }

    for mob in mob_blips {
        let dx = (mob.x - player_x) as f32;
        let dz = (mob.z - player_z) as f32;
        let (mx, mz) = world_offset_to_map(dx, dz, player_yaw, half);
        let color = if mob.hostile {
            [255, 45, 45, 255]
        } else {
            [255, 130, 175, 255]
        };
        stamp_marker(out, size, mx, mz, 2, color);
    }

    stamp_player_heading_up(out, size, half, half);
}

fn terrain_color_world(world: &VoxelWorld, wx: i32, wz: i32) -> [u8; 4] {
    if let Some((_, block)) = surface_at(world, wx, wz) {
        return block_minimap_color(block);
    }
    terrain_color_noise(&world.noise, wx, wz)
}

fn surface_at(world: &VoxelWorld, wx: i32, wz: i32) -> Option<(i32, BlockId)> {
    for y in (1..WORLD_HEIGHT).rev() {
        let block = world.peek_block(wx, y, wz);
        if block == BlockId::Water {
            return Some((y, block));
        }
        if block.solid() && block != BlockId::Bedrock {
            return Some((y, block));
        }
    }
    None
}

fn block_minimap_color(block: BlockId) -> [u8; 4] {
    match block {
        BlockId::Water => [36, 78, 140, 255],
        BlockId::Sand => [196, 176, 96, 255],
        BlockId::Snow => [214, 224, 236, 255],
        BlockId::Stone | BlockId::Cobblestone | BlockId::Bedrock => [110, 110, 110, 255],
        BlockId::Wood | BlockId::Planks => [160, 110, 55, 255],
        BlockId::Dirt => [120, 88, 48, 255],
        BlockId::Grass => [52, 118, 48, 255],
        BlockId::Lava | BlockId::Obsidian => [180, 60, 30, 255],
        BlockId::Glass => [140, 180, 210, 255],
        _ => [80, 80, 80, 255],
    }
}

fn terrain_color_noise(noise: &NoiseGenerator, wx: i32, wz: i32) -> [u8; 4] {
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
    [52, 118, 48, 255]
}

fn draw_ring(out: &mut [u8], size: i32, half: i32) {
    let r = half as f32 - 1.0;
    let r_inner = r - 1.5;
    let center = half as f32;
    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 - center;
            let dy = py as f32 - center;
            let d = (dx * dx + dy * dy).sqrt();
            if d > r || d < r_inner {
                continue;
            }
            let idx = ((py * size + px) * 4) as usize;
            blend_pixel(out, idx, [120, 200, 255, 200]);
        }
    }
}

fn blend_pixel(out: &mut [u8], idx: usize, overlay: [u8; 4]) {
    let a = overlay[3] as f32 / 255.0;
    for c in 0..3 {
        let base = out[idx + c] as f32;
        out[idx + c] = (base * (1.0 - a) + overlay[c] as f32 * a) as u8;
    }
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

/// Player at center; wedge always points up (forward).
fn stamp_player_heading_up(out: &mut [u8], size: i32, cx: i32, cz: i32) {
    stamp_marker(out, size, cx, cz, 2, [80, 220, 255, 255]);
    for i in 0..6 {
        stamp_marker(out, size, cx, cz - 3 - i, 0, [235, 250, 255, 255]);
        if i > 0 {
            stamp_marker(out, size, cx - 1, cz - 3 - i, 0, [235, 250, 255, 255]);
            stamp_marker(out, size, cx + 1, cz - 3 - i, 0, [235, 250, 255, 255]);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn draw_nav_hud(
    mut contexts: EguiContexts,
    ui_state: Res<GameUiState>,
    mut nav: ResMut<NavHud>,
    mut texture_cache: Local<Option<egui::TextureHandle>>,
) {
    if ui_state.screen != crate::menu::MenuScreen::Playing {
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

    let display = MINIMAP_DISPLAY_PX;
    egui::Area::new(egui::Id::new("minimap_hud"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, 14.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("MAP")
                        .strong()
                        .size(13.0)
                        .color(egui::Color32::from_rgb(200, 230, 255)),
                );
                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(8, 12, 20, 220))
                    .stroke(egui::Stroke::new(
                        2.0,
                        egui::Color32::from_rgb(100, 180, 255),
                    ))
                    .inner_margin(egui::Margin::same(4.0));
                frame.show(ui, |ui| {
                    if let Some(tex) = texture_cache.as_ref() {
                        ui.image((tex.id(), egui::vec2(display, display)));
                    }
                });
                ui.label(
                    egui::RichText::new("▲ ahead · gold village · red enemy · pink animal")
                        .size(10.0)
                        .color(egui::Color32::from_rgb(170, 180, 200)),
                );
            });
        });
}

#[cfg(target_arch = "wasm32")]
fn push_minimap_html(rgba: &[u8]) {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = ncUpdateMinimap)]
        fn nc_update_minimap(rgba: &[u8]);
    }

    nc_update_minimap(rgba);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::VoxelWorld;

    #[test]
    fn minimap_buffer_matches_size() {
        let world = VoxelWorld::new(42);
        let mut buf = vec![0u8; MINIMAP_SIZE * MINIMAP_SIZE * 4];
        build_minimap(&mut buf, &world, &[], &[], 0, 0, 0.0);
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
    fn heading_up_places_forward_at_top() {
        let (mx, mz) = world_offset_to_map(0.0, -10.0, 0.0, 40);
        assert_eq!(mx, 40);
        assert!(mz < 40, "ahead should map above center");
    }

    #[test]
    fn heading_up_matches_player_forward_at_yaw_zero() {
        // Player forward at yaw=0 is -Z; walking forward decreases z.
        let (mx, mz) = world_offset_to_map(0.0, -20.0, 0.0, 40);
        assert_eq!(mx, 40, "dead ahead stays centered");
        assert_eq!(mz, 20, "ahead should be toward top of map");
    }

    #[test]
    fn map_pixel_round_trips_world_offset() {
        let half = 40;
        for (dx, dz) in [(0, -20), (15, 0), (-8, 12)] {
            let (mx, mz) = world_offset_to_map(dx as f32, dz as f32, 0.0, half);
            let right = mx - half;
            let ahead = half - mz;
            assert_eq!(right, dx, "right axis at yaw 0");
            assert_eq!(ahead, -dz, "ahead axis at yaw 0");
            let rdx = right as f32;
            let rad = ahead as f32;
            let wx = rdx.round() as i32;
            let wz = -rad.round() as i32;
            assert_eq!(wx, dx);
            assert_eq!(wz, dz);
        }
    }
}
