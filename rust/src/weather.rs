use crate::config::DAY_LENGTH_SECS;
use bevy::prelude::*;
use bevy::pbr::DistanceFog;

#[derive(Resource)]
pub struct DayNight {
    pub time: f32,
}

impl Default for DayNight {
    fn default() -> Self {
        // Start at mid-morning so the sky reads as daytime immediately.
        Self {
            time: DAY_LENGTH_SECS * 0.25,
        }
    }
}

#[derive(Component)]
pub struct SunLight;

#[derive(Component)]
pub struct MoonLight;

#[derive(Clone, Copy, Debug)]
pub struct CelestialState {
    pub cycle: f32,
    pub sun: f32,
    pub sun_angle: f32,
    pub moon_angle: f32,
    pub sun_elevation: f32,
    pub moon_elevation: f32,
    pub daylight: f32,
    pub moonlight: f32,
    pub sun_forward: Vec3,
    pub moon_forward: Vec3,
}

pub fn celestial_state(day: &DayNight) -> CelestialState {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun = (cycle * std::f32::consts::TAU).sin();
    let sun_angle = cycle * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let moon_angle = sun_angle + std::f32::consts::PI;
    let sun_forward = Quat::from_rotation_x(sun_angle) * Vec3::NEG_Z;
    let moon_forward = Quat::from_rotation_x(moon_angle) * Vec3::NEG_Z;
    CelestialState {
        cycle,
        sun,
        sun_angle,
        moon_angle,
        sun_elevation: sun_forward.y,
        moon_elevation: moon_forward.y,
        daylight: sun.max(0.0),
        moonlight: (-sun).max(0.0),
        sun_forward,
        moon_forward,
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 >= edge1 {
        return if x >= edge0 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn sky_color(cycle: f32) -> Color {
    let sun = (cycle * std::f32::consts::TAU).sin();
    if sun < -0.15 {
        Color::srgb(0.06, 0.08, 0.20)
    } else if sun < 0.2 {
        let t = (sun + 0.15) / 0.35;
        Color::srgb(0.10 + t * 0.35, 0.08 + t * 0.54, 0.18 + t * 0.72)
    } else {
        let t = sun.clamp(0.0, 1.0);
        Color::srgb(0.45 + t * 0.08, 0.68 + t * 0.12, 0.95 - t * 0.08)
    }
}

pub fn update_day_night(
    time: Res<Time>,
    mut day: ResMut<DayNight>,
    mut clear: ResMut<ClearColor>,
    mut fog: Query<&mut DistanceFog, With<crate::player::PlayerCamera>>,
) {
    day.time += time.delta_secs();
    let state = celestial_state(&day);
    let mut sky = sky_color(state.cycle);

    // Warm sunrise / sunset tint on the horizon color.
    let sunset = (1.0 - (state.sun_elevation / 0.28).clamp(0.0, 1.0)) * state.daylight;
    if sunset > 0.01 {
        let warm = Color::srgb(0.98, 0.55, 0.22);
        sky = sky.mix(&warm, sunset * 0.42);
    }

    clear.0 = sky;
    if let Ok(mut fog) = fog.get_single_mut() {
        fog.color = sky;
    }
}

pub fn update_lights(
    day: Res<DayNight>,
    mut sun_lights: Query<(&mut DirectionalLight, &mut Transform), (With<SunLight>, Without<MoonLight>)>,
    mut moon_lights: Query<(&mut DirectionalLight, &mut Transform), (With<MoonLight>, Without<SunLight>)>,
    mut ambient: ResMut<AmbientLight>,
) {
    let state = celestial_state(&day);
    let sun_up = smoothstep(0.0, 0.22, state.sun_elevation);
    let moon_up = smoothstep(0.0, 0.18, state.moon_elevation);

    for (mut light, mut transform) in sun_lights.iter_mut() {
        let sunset = (1.0 - (state.sun_elevation / 0.32).clamp(0.0, 1.0)) * state.daylight;
        light.illuminance = (400.0 + state.daylight * 12000.0) * sun_up;
        light.color = Color::srgb(
            1.0,
            0.98 - sunset * 0.28,
            0.92 - sunset * 0.52,
        );
        transform.rotation = Quat::from_rotation_x(state.sun_angle);
    }

    for (mut light, mut transform) in moon_lights.iter_mut() {
        light.illuminance = (120.0 + state.moonlight * 3600.0) * moon_up;
        light.color = Color::srgb(0.68, 0.82, 1.0);
        transform.rotation = Quat::from_rotation_x(state.moon_angle);
    }

    let day_ambient = 180.0 + state.daylight * 680.0;
    let night_ambient = 140.0 + state.moonlight * 380.0;
    let mut brightness = day_ambient + night_ambient;
    if cfg!(target_arch = "wasm32") {
        // Keep nights readable on mobile without washing out stars/sky.
        let floor = 90.0 + state.daylight * 520.0 + state.moonlight * 120.0;
        brightness = brightness.max(floor);
    }
    ambient.brightness = brightness;
    ambient.color = Color::srgb(
        0.50 + state.daylight * 0.28 + state.moonlight * 0.08 + (1.0 - state.daylight) * 0.04,
        0.54 + state.daylight * 0.20 + state.moonlight * 0.10,
        0.72 + state.daylight * 0.08 + state.moonlight * 0.26,
    );
}

pub fn update_chunk_material_lighting(
    day: Res<DayNight>,
    chunk_mat: Res<crate::meshing::ChunkMaterial>,
    mut materials: ResMut<Assets<crate::chunk_material::VoxelChunkMaterial>>,
) {
    let state = celestial_state(&day);
    if let Some(mat) = materials.get_mut(&chunk_mat.0) {
        mat.sun_dir = Vec4::new(
            state.sun_forward.x,
            state.sun_forward.y,
            state.sun_forward.z,
            state.daylight,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_crosses_from_horizon_to_zenith() {
        let mut day = DayNight { time: 0.0 };
        let midnight = celestial_state(&day);
        assert!(midnight.sun_elevation < -0.8);
        assert!(midnight.moon_elevation > 0.9);

        day.time = DAY_LENGTH_SECS * 0.5;
        let noon = celestial_state(&day);
        assert!(noon.sun_elevation > 0.95);
        assert!(noon.moon_elevation < -0.8);

        day.time = DAY_LENGTH_SECS * 0.75;
        let dusk = celestial_state(&day);
        assert!(dusk.sun_elevation.abs() < 0.1);
        assert!(dusk.moon_elevation.abs() < 0.1);
    }
}
