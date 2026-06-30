use crate::menu::{is_playing, GameUiState};
use crate::player::PlayerState;
use crate::proc_mesh::box_mesh;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

/// World meshes (terrain, mobs, etc.) — default layer 0.
pub const WORLD_RENDER_LAYER: RenderLayers = RenderLayers::layer(0);
/// Player body is drawn on layer 1 so the first-person camera (layer 0) does not see it.
pub const PLAYER_BODY_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
pub struct PlayerBody;

#[derive(Component)]
pub struct PlayerLimb {
    pub side: LimbSide,
    pub kind: LimbKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LimbSide {
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LimbKind {
    Head,
    Torso,
    Arm,
    Leg,
}

#[derive(Resource)]
pub struct PlayerBodyAssets {
    pub skin: Handle<StandardMaterial>,
    pub shirt: Handle<StandardMaterial>,
    pub pants: Handle<StandardMaterial>,
    pub head_mesh: Handle<Mesh>,
    pub torso_mesh: Handle<Mesh>,
    pub limb_mesh: Handle<Mesh>,
}

pub fn setup_player_body(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.72, 0.58),
        perceptual_roughness: 1.0,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.45, 0.78),
        perceptual_roughness: 1.0,
        ..default()
    });
    let pants = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.32),
        perceptual_roughness: 1.0,
        ..default()
    });

    let head_mesh = meshes.add(box_mesh(
        Vec3::new(-0.2, 0.0, -0.2),
        Vec3::new(0.2, 0.4, 0.2),
    ));
    let torso_mesh = meshes.add(box_mesh(
        Vec3::new(-0.25, 0.0, -0.15),
        Vec3::new(0.25, 0.6, 0.15),
    ));
    let limb_mesh = meshes.add(box_mesh(
        Vec3::new(-0.12, 0.0, -0.12),
        Vec3::new(0.12, 0.6, 0.12),
    ));

    commands.insert_resource(PlayerBodyAssets {
        skin: skin.clone(),
        shirt: shirt.clone(),
        pants: pants.clone(),
        head_mesh: head_mesh.clone(),
        torso_mesh: torso_mesh.clone(),
        limb_mesh: limb_mesh.clone(),
    });

    commands
        .spawn((
            PlayerBody,
            Transform::default(),
            Visibility::Hidden,
            GlobalTransform::default(),
        ))
        .with_children(|root| {
            root.spawn((
                Mesh3d(torso_mesh),
                MeshMaterial3d(shirt.clone()),
                Transform::from_xyz(0.0, 0.9, 0.0),
                PLAYER_BODY_RENDER_LAYER,
                PlayerLimb {
                    side: LimbSide::Center,
                    kind: LimbKind::Torso,
                },
            ));
            root.spawn((
                Mesh3d(head_mesh),
                MeshMaterial3d(skin.clone()),
                Transform::from_xyz(0.0, 1.55, 0.0),
                PLAYER_BODY_RENDER_LAYER,
                PlayerLimb {
                    side: LimbSide::Center,
                    kind: LimbKind::Head,
                },
            ));
            for (side, x) in [(LimbSide::Left, -0.38), (LimbSide::Right, 0.38)] {
                root.spawn((
                    Mesh3d(limb_mesh.clone()),
                    MeshMaterial3d(shirt.clone()),
                    Transform::from_xyz(x, 1.35, 0.0),
                    PLAYER_BODY_RENDER_LAYER,
                    PlayerLimb {
                        side,
                        kind: LimbKind::Arm,
                    },
                ));
            }
            for (side, x) in [(LimbSide::Left, -0.14), (LimbSide::Right, 0.14)] {
                root.spawn((
                    Mesh3d(limb_mesh.clone()),
                    MeshMaterial3d(pants.clone()),
                    Transform::from_xyz(x, 0.6, 0.0),
                    PLAYER_BODY_RENDER_LAYER,
                    PlayerLimb {
                        side,
                        kind: LimbKind::Leg,
                    },
                ));
            }
        });
}

pub fn update_player_body(
    ui: Res<GameUiState>,
    player: Res<PlayerState>,
    mut body: Query<(&mut Transform, &mut Visibility), With<PlayerBody>>,
    mut limbs: Query<(&PlayerLimb, &mut Transform), Without<PlayerBody>>,
) {
    let Ok((mut root, mut vis)) = body.get_single_mut() else {
        return;
    };

    *vis = if is_playing(&ui) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    root.translation = player.position;
    root.rotation = Quat::from_rotation_y(player.yaw);

    let moving = player.velocity.x.abs() + player.velocity.z.abs() > 0.4 && player.on_ground;
    let swing = if moving {
        player.walk_bob_phase.sin() * 0.55
    } else {
        0.0
    };

    for (limb, mut transform) in limbs.iter_mut() {
        match limb.kind {
            LimbKind::Head => {}
            LimbKind::Torso => {}
            LimbKind::Arm => {
                let sign = if limb.side == LimbSide::Left { 1.0 } else { -1.0 };
                transform.rotation = Quat::from_rotation_x(swing * sign);
            }
            LimbKind::Leg => {
                let sign = if limb.side == LimbSide::Left { -1.0 } else { 1.0 };
                transform.rotation = Quat::from_rotation_x(swing * sign);
            }
        }
    }
}
