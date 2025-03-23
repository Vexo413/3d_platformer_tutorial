use bevy::{
    color::palettes::css,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RapierPhysicsPlugin::<NoUserData>::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                manage_collisions,
                manage_position,
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
        .insert_resource(PlayerPhysics {
            speed: 1.0,
            ground_friction: 0.9,
            air_friction: 0.95,
            jump_force: 75.0,
            gravity: 1.0,
        })
        .run();
}

#[derive(Component)]
struct Player {
    velocity: Vec3,
    grounded: bool,
}

#[derive(Component)]
struct CameraArm;

#[derive(Component)]
struct GroundSensor;

#[derive(Resource)]
struct PlayerPhysics {
    speed: f32,
    ground_friction: f32,
    air_friction: f32,
    jump_force: f32,
    gravity: f32,
}

#[derive(Resource)]
struct MouseSettings {
    sensitivity: f32,
}

#[derive(Resource)]
struct CursorState {
    grabbed: bool,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 1.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(css::WHITE))),
        RigidBody::Fixed,
        Collider::cuboid(20.0, 0.5, 20.0),
        Transform::default(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.25, 0.25))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(css::BLUE))),
        RigidBody::Fixed,
        Collider::cuboid(5.0, 0.125, 0.125),
        Transform::from_xyz(5.0, 0.625, 5.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(css::RED))),
        RigidBody::Fixed,
        Collider::cuboid(1.0, 1.0, 1.0),
        Transform::from_xyz(10.0, 1.5, -5.0),
    ));

    commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(1.0, 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial::from_color(css::GREEN))),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(1.0, 1.0),
            Transform::from_xyz(0.0, 50.0, 0.0),
            KinematicCharacterController {
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5),
                    min_width: CharacterLength::Absolute(0.5),
                    include_dynamic_bodies: true,
                }),
                ..Default::default()
            },
            Player {
                velocity: Vec3::ZERO,
                grounded: false,
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((CameraArm, Transform::from_xyz(0.0, 0.0, 0.0)))
                .with_children(|parent| {
                    parent.spawn((Camera3d::default(), Transform::from_xyz(0.0, 1.0, 10.0)));
                });
            parent.spawn((
                Collider::ball(0.1),
                Sensor,
                Transform::from_xyz(0.0, -2.1, 0.0),
                GroundSensor,
                ActiveEvents::COLLISION_EVENTS,
                ActiveCollisionTypes::all(),
            ));
        });
}

fn manage_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut query: Query<&mut Player>,
    sensor_query: Query<Entity, With<GroundSensor>>,
) {
    for event in collision_events.read() {
        match event {
            CollisionEvent::Started(entity1, entity2, _) => {
                if sensor_query.get(*entity1).is_ok() || sensor_query.get(*entity2).is_ok() {
                    if let Ok(mut player) = query.get_single_mut() {
                        player.grounded = true;
                    }
                }
            }
            CollisionEvent::Stopped(entity1, entity2, _) => {
                if sensor_query.get(*entity1).is_ok() || sensor_query.get(*entity2).is_ok() {
                    if let Ok(mut player) = query.get_single_mut() {
                        player.grounded = false;
                    }
                }
            }
        }
    }
}

fn manage_position(
    time: Res<Time>,
    mut query: Query<(&mut KinematicCharacterController, &mut Player, &Transform)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    player_physics: Res<PlayerPhysics>,
) {
    let (mut controller, mut player, player_transform) = query.single_mut();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::Space) && player.grounded {
        player.velocity.y = player_physics.jump_force * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::KeyW) {
        direction.z = -1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z = 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x = -1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x = 1.0;
    }

    direction = direction.normalize_or_zero();

    let rotated_direction = player_transform.rotation * direction;

    player.velocity += rotated_direction * player_physics.speed * time.delta_secs();
    if player.grounded {
        player.velocity *= player_physics.ground_friction;
    } else {
        player.velocity *= player_physics.air_friction;
    }
    player.velocity.y -= player_physics.gravity * time.delta_secs();
    controller.translation = Some(player.velocity);
}

fn manage_rotation(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<CameraArm>, Without<Player>)>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_settings: Res<MouseSettings>,
    cursor_state: Res<CursorState>,
) {
    let mut player_transform = query.single_mut();
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
