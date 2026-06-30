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
pub const MINIMAP_SIZE: usize = 64;
pub const MINIMAP_RADIUS: i32 = 64;
const LANDMARK_SEARCH_RADIUS: i32 = 1024;
const MINIMAP_REFRESH_SECS: f32 = 0.15;

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
    pub nearest_village: Option<Landmark>,
    pub nearest_volcano: Option<Landmark>,
    pub village_hint: String,
    pub refresh_timer: f32,
}

impl Default for NavHud {
    fn default() -> Self {
        Self {
            minimap_rgba: vec![0; MINIMAP_SIZE * MINIMAP_SIZE * 4],
            minimap_dirty: true,
            landmarks: Vec::new(),
            mob_blips: Vec::new(),
            nearest_village: None,
            nearest_volcano: None,
            village_hint: String::new(),
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
        .find(|l| l.kind == LandmarkKind::Village && !l.discovered && l.distance < 400.0)
    {
        crate::chunk_gen::ensure_settlements_near(&mut world.inner, village.x, village.z, 128);
    }

    nav.landmarks = collect_landmarks(&world.inner, px, pz, LANDMARK_SEARCH_RADIUS);
    nav.mob_blips = collect_mob_blips(&mobs);

    nav.nearest_village = nav
        .landmarks
        .iter()
        .copied()
        .filter(|l| l.kind == LandmarkKind::Village && l.discovered)
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
        .or_else(|| {
            nav.landmarks
                .iter()
                .copied()
                .filter(|l| l.kind == LandmarkKind::Village)
                .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
        });

    nav.nearest_volcano = nav
        .landmarks
        .iter()
        .copied()
        .filter(|l| l.kind == LandmarkKind::Volcano)
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));

    nav.village_hint = village_hint_text(nav.nearest_village);

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
    push_nav_html(&nav, player.yaw);
}

fn village_hint_text(village: Option<Landmark>) -> String {
    let Some(village) = village else {
        return "No village on map — keep exploring".to_string();
    };
    if village.discovered {
        if village.distance < 40.0 {
            return format!("Village nearby — look for houses ({:.0}m)", village.distance);
        }
        return format!("Follow gold arrow → Village ({:.0}m)", village.distance);
    }
    format!("Uncharted village ~{:.0}m — head that way", village.distance)
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
        let dist = block_distance(px, pz, center_x, center_z);
        if dist <= radius as f32 {
            landmarks.push(Landmark {
                kind: LandmarkKind::Village,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
                discovered: true,
            });
        }
    }

    for &(center_x, center_z) in &world.volcano_centers {
        let dist = block_distance(px, pz, center_x, center_z);
        if dist <= radius as f32 {
            landmarks.push(Landmark {
                kind: LandmarkKind::Volcano,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
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
            let dist = block_distance(px, pz, center_x, center_z);
            if dist > radius as f32 {
                continue;
            }
            landmarks.push(Landmark {
                kind: LandmarkKind::Village,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
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
            let dist = block_distance(px, pz, center_x, center_z);
            if dist > radius as f32 {
                continue;
            }
            landmarks.push(Landmark {
                kind: LandmarkKind::Volcano,
                x: center_x,
                z: center_z,
                distance: dist,
                bearing: world_bearing(px, pz, center_x, center_z),
                discovered: false,
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
    world: &VoxelWorld,
    landmarks: &[Landmark],
    mob_blips: &[MobBlip],
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
            let rgba = terrain_color_world(world, wx, wz);
            out[idx..idx + 4].copy_from_slice(&rgba);
        }
    }

    draw_grid(out, size);

    for landmark in landmarks {
        let mx = landmark.x - player_x + half;
        let mz = landmark.z - player_z + half;
        let color = match landmark.kind {
            LandmarkKind::Village if landmark.discovered => [255, 220, 60, 255],
            LandmarkKind::Village => [255, 200, 60, 140],
            LandmarkKind::Volcano if landmark.discovered => [255, 80, 50, 255],
            LandmarkKind::Volcano => [220, 60, 40, 150],
        };
        let radius = if landmark.kind == LandmarkKind::Village { 3 } else { 2 };
        stamp_marker(out, size, mx, mz, radius, color);
    }

    for mob in mob_blips {
        let mx = mob.x - player_x + half;
        let mz = mob.z - player_z + half;
        let color = if mob.hostile {
            [255, 50, 50, 255]
        } else {
            [255, 140, 180, 255]
        };
        stamp_marker(out, size, mx, mz, 1, color);
    }

    stamp_player(out, size, half, half, player_yaw);
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
        BlockId::Wood | BlockId::Planks => [140, 100, 55, 255],
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

fn draw_grid(out: &mut [u8], size: i32) {
    for i in 0..size {
        if i % 16 != 0 {
            continue;
        }
        for j in 0..size {
            let idx = ((j * size + i) * 4) as usize;
            blend_pixel(out, idx, [255, 255, 255, 40]);
            let idx = ((i * size + j) * 4) as usize;
            blend_pixel(out, idx, [255, 255, 255, 40]);
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

fn stamp_player(out: &mut [u8], size: i32, cx: i32, cz: i32, yaw: f32) {
    stamp_marker(out, size, cx, cz, 2, [80, 220, 255, 255]);
    let forward = Vec2::new(-yaw.sin(), -yaw.cos());
    for step in 1..=5 {
        let tip_x = cx + (forward.x * step as f32).round() as i32;
        let tip_z = cz + (forward.y * step as f32).round() as i32;
        stamp_marker(out, size, tip_x, tip_z, 0, [220, 245, 255, 255]);
    }
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
                ui.label(
                    egui::RichText::new(&nav.village_hint)
                        .size(12.0)
                        .color(egui::Color32::from_rgb(235, 235, 245)),
                );
                draw_wayfinder(ui, nav, player_yaw);
                ui.add_space(4.0);
                if let Some(tex) = texture_cache {
                    let size = MINIMAP_SIZE as f32;
                    ui.image((tex.id(), egui::vec2(size, size)));
                }
                ui.label(
                    egui::RichText::new("▲ N   ● you   ■ houses   ● pink=animals   ● red=enemies")
                        .small()
                        .color(egui::Color32::from_rgb(180, 180, 195)),
                );
            });
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_wayfinder(ui: &mut egui::Ui, nav: &NavHud, player_yaw: f32) {
    let panel_w = MINIMAP_SIZE as f32;
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(panel_w, panel_w * 0.38),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = rect.width() * 0.36;

    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(220, 220, 230, 200)),
    );
    painter.text(
        rect.center_top() + egui::vec2(0.0, 2.0),
        egui::Align2::CENTER_TOP,
        "N",
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(200, 200, 210),
    );
    painter.circle_filled(
        center,
        2.5,
        egui::Color32::from_rgb(80, 220, 255),
    );

    if let Some(village) = nav.nearest_village {
        let rel = relative_bearing(player_yaw, village.bearing);
        let arrow = egui::vec2(rel.sin(), -rel.cos()) * radius * 0.85;
        let tip = center + arrow;
        let wing = egui::vec2(-arrow.y, arrow.x).normalized() * 6.0;
        let color = if village.discovered {
            egui::Color32::from_rgb(255, 210, 80)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 210, 80, 140)
        };
        painter.line_segment([center + wing * 0.3, tip], egui::Stroke::new(3.0, color));
        painter.line_segment([center - wing * 0.3, tip], egui::Stroke::new(3.0, color));
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
            village_discovered: bool,
            volcano_dist: f32,
            volcano_bearing: f32,
            volcano_active: bool,
            hint: &str,
            mob_csv: &str,
            rgba: &[u8],
        );
    }

    let (v_dist, v_bearing, v_active, v_discovered) = nav
        .nearest_village
        .map(|v| (v.distance, relative_bearing(player_yaw, v.bearing), true, v.discovered))
        .unwrap_or((0.0, 0.0, false, false));
    let (vol_dist, vol_bearing, vol_active) = nav
        .nearest_volcano
        .map(|v| (v.distance, relative_bearing(player_yaw, v.bearing), true))
        .unwrap_or((0.0, 0.0, false));

    let mob_csv = nav
        .mob_blips
        .iter()
        .map(|m| format!("{},{}", m.x, m.z))
        .collect::<Vec<_>>()
        .join(";");

    nc_update_nav_hud(
        player_yaw,
        v_dist,
        v_bearing,
        v_active,
        v_discovered,
        vol_dist,
        vol_bearing,
        vol_active,
        &nav.village_hint,
        &mob_csv,
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
    fn relative_bearing_is_zero_when_facing_target() {
        let bearing = world_bearing(0, 0, 0, -20);
        let rel = relative_bearing(bearing, bearing);
        assert!(rel.abs() < 0.001);
    }
}
