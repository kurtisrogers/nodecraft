use crate::config::DAY_LENGTH_SECS;
use bevy::prelude::*;

#[derive(Resource)]
pub struct DayNight {
    pub time: f32,
}

impl Default for DayNight {
    fn default() -> Self {
        Self { time: 0.0 }
    }
}

pub fn update_day_night(time: Res<Time>, mut day: ResMut<DayNight>, mut clear: ResMut<ClearColor>) {
    day.time += time.delta_secs();
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun = (cycle * std::f32::consts::TAU).sin();
    let night = cycle > 0.5;

    let sky = if night {
        Color::srgb(0.04, 0.04, 0.13)
    } else if sun > 0.0 {
        Color::hsl(0.58, 0.6, 0.45 + sun * 0.25)
    } else {
        Color::srgb(0.10, 0.06, 0.25)
    };
    clear.0 = sky;
}

pub fn update_lights(
    day: Res<DayNight>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform), With<DirectionalLight>>,
) {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun = (cycle * std::f32::consts::TAU).sin().max(0.1);
    for (mut light, mut transform) in lights.iter_mut() {
        light.illuminance = sun * 12000.0;
        transform.rotation = Quat::from_rotation_x(-0.8);
    }
}
