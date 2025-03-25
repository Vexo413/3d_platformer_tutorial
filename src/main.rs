use avian3d::prelude::*;
use bevy::{
    color::palettes::css,
    input::mouse::MouseMotion,
    prelude::*,
    render::mesh::Mesh,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_skein::SkeinPlugin;
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            SkeinPlugin::default(),
            TnuaAvian3dPlugin::new(Update),
            TnuaControllerPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                manage_position.in_set(TnuaUserControlsSystemSet),
                manage_rotation,
                manage_cursor_lock,
            ),
        )
        .insert_resource(AmbientLight {
            brightness: 2000.0,
            color: Color::WHITE,
        })
        .insert_resource(MouseSettings { sensitivity: 0.5 })
        .insert_resource(CursorState { grabbed: false })
        .insert_resource(GameState { level: 1 })
        .run();
}

#[derive(Component)]
struct CameraArm;

#[derive(Resource)]
struct MouseSettings {
    sensitivity: f32,
}

#[derive(Resource)]
struct CursorState {
    grabbed: bool,
}

#[derive(Resource)]
struct GameState {
    level: u32,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Ground;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    game_state: Res<GameState>,
) {
    match game_state.level {
        1 => {
            commands.spawn(SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("levels.gltf")),
            ));
        }
        _ => {}
    }

    commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(1.0, 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial::from_color(css::GREEN))),
            Collider::capsule(1.0, 2.0),
            Transform::from_xyz(0.0, 5.0, 0.0),
            TnuaController::default(),
            RigidBody::Dynamic,
            TnuaAvian3dSensorShape(Collider::cylinder(0.24, 0.0)),
            // Tnua can fix the rotation, but the character will still get rotated before it can do so.
            // By locking the rotation we can prevent this.
            LockedAxes::ROTATION_LOCKED,
        ))
        .with_children(|parent| {
            parent
                .spawn((CameraArm, Transform::from_xyz(0.0, 0.0, 0.0)))
                .with_children(|parent| {
                    parent.spawn((Camera3d::default(), Transform::from_xyz(0.0, 1.0, 10.0)));
                });
        });
}

fn manage_position(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut TnuaController, &mut Transform)>,
) {
    let Ok((mut controller, mut transform)) = query.get_single_mut() else {
        return;
    };

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction = Vec3::NEG_Z;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction = Vec3::Z;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction = Vec3::NEG_X;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction = Vec3::X;
    }

    let rotated_direction = transform.rotation * direction.normalize_or_zero();
    controller.basis(TnuaBuiltinWalk {
        desired_velocity: rotated_direction * 20.0,
        float_height: 2.0,
        ..Default::default()
    });

    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 15.0,
            ..Default::default()
        });
    }

    if transform.translation.y < -20.0 {
        transform.translation = Vec3::new(0.0, 5.0, 0.0)
    }
}

fn manage_rotation(
    time: Res<Time>,
    mut player_query: Query<&mut Transform, With<TnuaController>>,
    mut camera_query: Query<&mut Transform, (With<CameraArm>, Without<TnuaController>)>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_settings: Res<MouseSettings>,
    cursor_state: Res<CursorState>,
) {
    let mut player_transform = player_query.single_mut();
    if cursor_state.grabbed {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            for event in mouse_motion_events.read() {
                let delta = event.delta;
                player_transform
                    .rotate_y(-delta.x * mouse_settings.sensitivity * time.delta_secs());

                camera_transform
                    .rotate_x(-delta.y * mouse_settings.sensitivity * time.delta_secs());
                let pitch = camera_transform
                    .rotation
                    .to_euler(EulerRot::XYZ)
                    .0
                    .clamp(-1.25, 0.25);
                camera_transform.rotation = Quat::from_euler(EulerRot::XYZ, pitch, 0.0, 0.0);
            }
        }
    }
}

fn manage_cursor_lock(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_state: ResMut<CursorState>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let mut primary_window = q_windows.single_mut();
        primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor_options.visible = false;
        cursor_state.grabbed = true;
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        let mut primary_window = q_windows.single_mut();
        primary_window.cursor_options.grab_mode = CursorGrabMode::None;
        primary_window.cursor_options.visible = true;
        cursor_state.grabbed = false;
    }
}
