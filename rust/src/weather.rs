use crate::config::DAY_LENGTH_SECS;
use bevy::prelude::*;
use bevy_pbr::DistanceFog;

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
    pub daylight: f32,
    pub moonlight: f32,
}

pub fn celestial_state(day: &DayNight) -> CelestialState {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun = (cycle * std::f32::consts::TAU).sin();
    let sun_angle = cycle * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let moon_angle = sun_angle + std::f32::consts::PI;
    CelestialState {
        cycle,
        sun,
        sun_angle,
        moon_angle,
        daylight: sun.max(0.0),
        moonlight: (-sun).max(0.0),
    }
}

fn sky_color(cycle: f32) -> Color {
    let sun = (cycle * std::f32::consts::TAU).sin();
    if sun < -0.15 {
        Color::srgb(0.05, 0.07, 0.18)
    } else if sun < 0.2 {
        let t = (sun + 0.15) / 0.35;
        Color::srgb(0.12 + t * 0.33, 0.10 + t * 0.52, 0.22 + t * 0.70)
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
    let sky = sky_color(celestial_state(&day).cycle);
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

    for (mut light, mut transform) in sun_lights.iter_mut() {
        light.illuminance = 2000.0 + state.daylight * 10000.0;
        light.color = Color::srgb(1.0, 0.98, 0.92);
        transform.rotation = Quat::from_rotation_x(state.sun_angle);
    }

    for (mut light, mut transform) in moon_lights.iter_mut() {
        light.illuminance = 250.0 + state.moonlight * 2400.0;
        light.color = Color::srgb(0.72, 0.84, 1.0);
        transform.rotation = Quat::from_rotation_x(state.moon_angle);
    }

    ambient.brightness = 80.0 + state.daylight * 520.0 + state.moonlight * 140.0;
    ambient.color = Color::srgb(
        0.55 + state.daylight * 0.2 + state.moonlight * 0.08,
        0.58 + state.daylight * 0.15 + state.moonlight * 0.1,
        0.72 + state.daylight * 0.1 + state.moonlight * 0.22,
    );
}
