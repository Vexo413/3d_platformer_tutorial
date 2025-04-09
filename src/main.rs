use avian3d::{
    PhysicsPlugins,
    prelude::{Collider, CollisionStarted, LockedAxes, RigidBody},
};
use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    asset::AssetServer,
    color::Color,
    input::{ButtonInput, mouse::MouseMotion},
    math::{EulerRot, Quat, Vec3},
    pbr::AmbientLight,
    prelude::{
        BuildChildren, Camera3d, ChildBuild, Commands, Component, DespawnRecursiveExt, Entity,
        EventReader, IntoSystemConfigs, KeyCode, MouseButton, Query, ReflectComponent, Res, ResMut,
        Resource, Transform, With, Without,
    },
    reflect::Reflect,
    scene::SceneRoot,
    time::{Time, Virtual},
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_gltf::GltfAssetLabel;
use bevy_skein::SkeinPlugin;
use bevy_tnua::{
    TnuaUserControlsSystemSet,
    prelude::{TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController, TnuaControllerPlugin},
};
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};
fn main() {
    App::new()
        .register_type::<GameObject>()
        .register_type::<Goal>()
        .register_type::<Spike>()
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
                manage_collisions,
                rotate_goal,
            ),
        )
        .insert_resource(AmbientLight {
            brightness: 2500.0,
            color: Color::WHITE,
        })
        .insert_resource(MouseSettings { sensitivity: 0.5 })
        .insert_resource(CursorState { grabbed: false })
        .insert_resource(GameState { level: 0 })
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
    level: usize,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct GameObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Goal;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Spike;

fn setup(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    game_state: Res<GameState>,
) {
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(game_state.level).from_asset("levels.glb"),
    )));

    commands
        .spawn((
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("player.glb"))),
            //Mesh3d(meshes.add(Capsule3d::new(1.0, 2.0))),
            /*MeshMaterial3d(
                materials.add(StandardMaterial::from_color(css::GREEN.with_alpha(0.25))),
            ),*/
            Collider::capsule(1.0, 2.0),
            Transform::from_xyz(0.0, 5.0, 0.0),
            TnuaController::default(),
            RigidBody::Dynamic,
            TnuaAvian3dSensorShape(Collider::cylinder(0.99, 0.0)),
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

    let rotated_direction;
    if transform.translation.y < -20.0 {
        transform.translation = Vec3::new(0.0, 5.0, 0.0);
        transform.rotation = Quat::IDENTITY;
        rotated_direction = Vec3::ZERO;
    } else {
        rotated_direction = transform.rotation * direction.normalize_or_zero();
    }
    controller.basis(TnuaBuiltinWalk {
        desired_velocity: rotated_direction * 20.0,
        float_height: 2.2,
        ..Default::default()
    });

    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 10.0,
            ..Default::default()
        });
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

fn manage_collisions(
    mut commands: Commands,
    object_query: Query<Entity, With<GameObject>>,
    mut player_query: Query<(Entity, &mut Transform), With<TnuaController>>,
    goal_query: Query<Entity, With<Goal>>,
    mut collision_started: EventReader<CollisionStarted>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<GameState>,
    mut time: ResMut<Time<Virtual>>,
) {
    for CollisionStarted(entity1, entity2) in collision_started.read() {
        if (player_query.get(*entity1).is_ok() && goal_query.get(*entity2).is_ok())
            || (player_query.get(*entity2).is_ok() && goal_query.get(*entity1).is_ok())
        {
            time.pause();
            for entity in object_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            game_state.level += 1;
            commands.spawn(SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(game_state.level).from_asset("levels.glb"),
            )));
            if let Ok((_, mut transform)) = player_query.get_single_mut() {
                transform.translation = Vec3::new(0.0, 5.0, 0.0);
                transform.rotation = Quat::IDENTITY;
            }
            time.unpause();
        }
    }
}

fn rotate_goal(time: Res<Time>, mut query: Query<&mut Transform, With<Goal>>) {
    if let Ok(mut transform) = query.get_single_mut() {
        transform.rotate_y(5.0 * time.delta_secs());
    }
}
