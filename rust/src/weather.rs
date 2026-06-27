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
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sky = sky_color(cycle);
    clear.0 = sky;
    if let Ok(mut fog) = fog.get_single_mut() {
        fog.color = sky;
    }
}

pub fn update_lights(
    day: Res<DayNight>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform), With<DirectionalLight>>,
    mut ambient: ResMut<AmbientLight>,
) {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun = (cycle * std::f32::consts::TAU).sin();
    let daylight = sun.max(0.0);
    let sun_angle = cycle * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;

    for (mut light, mut transform) in lights.iter_mut() {
        light.illuminance = 2000.0 + daylight * 10000.0;
        transform.rotation = Quat::from_rotation_x(sun_angle);
    }

    ambient.brightness = 80.0 + daylight * 520.0;
    ambient.color = Color::srgb(
        0.75 + daylight * 0.2,
        0.78 + daylight * 0.15,
        0.85 + daylight * 0.1,
    );
}
