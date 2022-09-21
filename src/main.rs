use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

mod map;
mod map_builder;

mod prelude {
    pub use bracket_lib::prelude::{
        Algorithm2D, BaseMap, DijkstraMap, DistanceAlg, Point, RandomNumberGenerator, Rect,
        SmallVec,
    };
    pub const SCREEN_WIDTH: i32 = 80;
    pub const SCREEN_HEIGHT: i32 = 50;
    pub const MAP_SIZE_WIDTH: usize = 14;
    pub const MAP_SIZE_HEIGHT: usize = 21;
    pub use crate::map::*;
    pub use crate::map_builder::*;
}

use prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
    GameOver,
}

#[derive(Component, Default)]
struct Player {
    entity: Option<Entity>,
    x: usize,
    y: usize,
    move_cooldown: Timer,
    velocity: Vec3,
    accel: f32,
    max_speed: f32,
    sensitivity: f32,
    friction: f32,
    pitch: f32,
    yaw: f32,
}

#[derive(Default)]
struct Game {
    map: Map,
    player: Player,
    score: i32,
    camera_should_focus: Vec3,
    camera_is_focus: Vec3,
}

const RESET_FOCUS: [f32; 3] = [
    MAP_SIZE_HEIGHT as f32 / 2.0,
    0.0,
    MAP_SIZE_WIDTH as f32 / 2.0 - 0.5,
];

fn main() {
    println!("Hello, world!");
    App::new()
        .init_resource::<Game>()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_state(GameState::Playing)
        .add_startup_system(setup_cameras)
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                //.with_system(move_player)
                // .with_system(camera_movement_system),
                .with_system(movement)
                .with_system(player_input)
        )
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(teardown))
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn setup_cameras(mut commands: Commands, mut game: ResMut<Game>) {
    game.camera_should_focus = Vec3::from(RESET_FOCUS);
    game.camera_is_focus = game.camera_should_focus;
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(
                -(MAP_SIZE_WIDTH as f32 / 2.0),
                2.0 * MAP_SIZE_HEIGHT as f32 / 3.0,
                MAP_SIZE_HEIGHT as f32 / 2.0 - 0.5,
        )
        .looking_at(game.camera_is_focus, Vec3::Y),
        ..default()
    });
    // commands
    //     .spawn()
    //     .insert_bundle(Camera3dBundle {
    //         transform: Transform::from_xyz(
    //             -(MAP_SIZE_WIDTH as f32 / 2.0),
    //             2.0 * MAP_SIZE_HEIGHT as f32 / 3.0,
    //             MAP_SIZE_HEIGHT as f32 / 2.0 - 0.5,
    //         )
    //         .looking_at(game.camera_is_focus, Vec3::Y),
    //         ..default()
    //     })
    //     .insert(FlyCamera::default());
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut game: ResMut<Game>) {
    let mut rng = RandomNumberGenerator::new();
    let map_builder = MapBuilder::new(&mut rng);
    game.score = 0;
    game.player.x = MAP_SIZE_WIDTH / 2;
    game.player.y = MAP_SIZE_HEIGHT / 2;
    game.player.move_cooldown = Timer::from_seconds(0.1, false);
    game.map = map_builder.map;

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 10.0, 4.0),
        point_light: PointLight {
            intensity: 3000.0,
            shadows_enabled: true,
            range: 30.0,
            ..default()
        },
        ..default()
    });

    let cell_scene = asset_server.load("resources/tile.glb#Scene0");
    game.map
        .tiles
        .iter()
        .enumerate()
        .for_each(|(idx, tile)| match tile {
            TileType::Wall => {
                commands.spawn_bundle(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        0.2,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                });
            }
            TileType::Floor => {
                commands.spawn_bundle(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        0.,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                });
            }
            TileType::Exit => {
                commands.spawn_bundle(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        -0.2,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                });
            }
        });

    game.player.entity = Some(
        commands
            .spawn_bundle(SceneBundle {
                transform: Transform {
                    translation: Vec3::new(game.player.x as f32, 0., game.player.y as f32),
                    rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                    ..default()
                },
                scene: asset_server.load("resources/alien.glb#Scene0"),
                ..default()
            })
            .id(),
    );

    // scoreboard
    commands.spawn_bundle(
        TextBundle::from_section(
            "Score:",
            TextStyle {
                font: asset_server.load("resources/FiraMono-Medium.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
            ..default()
        }),
    );
}

fn teardown(mut commands: Commands, entities: Query<Entity, Without<Camera>>) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
}

// control the game character
fn move_player(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut game: ResMut<Game>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
) {
    if game.player.move_cooldown.tick(time.delta()).finished() {
        let mut moved = false;
        let mut rotation = 0.0;

        if keyboard_input.pressed(KeyCode::Up) {
            if game.player.y < MAP_SIZE_HEIGHT - 1 {
                game.player.y += 1;
            }
            rotation = -std::f32::consts::FRAC_PI_2;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            if game.player.y > 0 {
                game.player.y -= 1;
            }
            rotation = std::f32::consts::FRAC_PI_2;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            if game.player.x < MAP_SIZE_WIDTH - 1 {
                game.player.x += 1;
            }
            rotation = std::f32::consts::PI;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Left) {
            if game.player.x > 0 {
                game.player.x -= 1;
            }
            rotation = 0.0;
            moved = true;
        }

        // move on the board
        if moved {
            game.player.move_cooldown.reset();
            *transforms.get_mut(game.player.entity.unwrap()).unwrap() = Transform {
                translation: Vec3::new(game.player.x as f32, 0., game.player.y as f32),
                rotation: Quat::from_rotation_y(rotation),
                ..default()
            };
        }
    }
}

// restart the game when pressing spacebar
fn gameover_keyboard(mut state: ResMut<State<GameState>>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Playing).unwrap();
    }
}

fn movement(
    time: Res<Time>,
    // mut camera_query: Query<(&mut FlyCamera, &mut Transform)>,
    // mut transforms: ParamSet<(Query<&mut Transform, With<Camera3d>>, Query<&Transform>)>,
    // mut game: ResMut<Game>,
    mut transforms: Query<(&WantsToMove, &mut Transform)>,
) {
    //let player_transforms = *transforms.get_mut(game.player.entity.unwrap()).unwrap().1;

    for (wants_to_move, mut transform, ) in transforms.iter_mut() {

        let mut velocity =  wants_to_move.velocity;
        
        let rotation = transform.rotation;
        let accel: Vec3 = (strafe_vector(&rotation) * wants_to_move.axis_h)
            + (forward_walk_vector(&rotation) * wants_to_move.axis_v)
            + (Vec3::Y * wants_to_move.axis_float);
        let accel: Vec3 = if accel.length() != 0.0 {
            accel.normalize() * wants_to_move.accel
        } else {
            Vec3::ZERO
        };

        let friction: Vec3 = if velocity.length() != 0.0 {
            velocity.normalize() * -1.0 * wants_to_move.friction
        } else {
            Vec3::ZERO
        };

        velocity += accel * time.delta_seconds();

        // clamp within max speed
        if velocity.length() > wants_to_move.max_speed {
           velocity = velocity.normalize() * wants_to_move.max_speed;
        }

        let delta_friction = friction * time.delta_seconds();

        velocity =
            if (velocity + delta_friction).signum() != velocity.signum() {
                Vec3::ZERO
            } else {
                velocity + delta_friction
            };

        transform.translation += velocity;
    }
}

fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut game: ResMut<Game>,
) {
    let (axis_h, axis_v, axis_float) = (
        movement_axis(&keyboard_input, KeyCode::D, KeyCode::A),
        movement_axis(&keyboard_input, KeyCode::S, KeyCode::W),
        movement_axis(&keyboard_input, KeyCode::LShift, KeyCode::LControl),
    );

    commands.entity(game.player.entity.unwrap()).insert(WantsToMove {
        axis_h,
        axis_v,
        axis_float,
        accel: 1.5,
        max_speed: 0.5,
        sensitivity: 3.0,
        friction: 1.0,
        pitch: 0.0,
        yaw: 0.0,
        velocity: Vec3::ZERO,

    });
}

#[derive(Component)]
struct WantsToMove {
    axis_h: f32,
    axis_v: f32,
    axis_float: f32,
    velocity: Vec3,
    accel: f32,
    max_speed: f32,
    sensitivity: f32,
    friction: f32,
    pitch: f32,
    yaw: f32,
}

// Camera
#[derive(Component)]
pub struct FlyCamera {
    /// The speed the FlyCamera accelerates at. Defaults to `1.0`
    pub accel: f32,
    /// The maximum speed the FlyCamera can move at. Defaults to `0.5`
    pub max_speed: f32,
    /// The sensitivity of the FlyCamera's motion based on mouse movement. Defaults to `3.0`
    pub sensitivity: f32,
    /// The amount of deceleration to apply to the camera's motion. Defaults to `1.0`
    pub friction: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub pitch: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub yaw: f32,
    /// The current velocity of the FlyCamera. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub velocity: Vec3,
    /// Key used to move forward. Defaults to <kbd>W</kbd>
    pub key_forward: KeyCode,
    /// Key used to move backward. Defaults to <kbd>S</kbd>
    pub key_backward: KeyCode,
    /// Key used to move left. Defaults to <kbd>A</kbd>
    pub key_left: KeyCode,
    /// Key used to move right. Defaults to <kbd>D</kbd>
    pub key_right: KeyCode,
    /// Key used to move up. Defaults to <kbd>Space</kbd>
    pub key_up: KeyCode,
    /// Key used to move forward. Defaults to <kbd>LShift</kbd>
    pub key_down: KeyCode,
    /// If `false`, disable keyboard control of the camera. Defaults to `true`
    pub enabled: bool,
}
impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            accel: 1.5,
            max_speed: 0.5,
            sensitivity: 3.0,
            friction: 1.0,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
            key_forward: KeyCode::W,
            key_backward: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::Space,
            key_down: KeyCode::LShift,
            enabled: true,
        }
    }
}

fn forward_vector(rotation: &Quat) -> Vec3 {
    rotation.mul_vec3(Vec3::Z).normalize()
}

fn forward_walk_vector(rotation: &Quat) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x, 0.0, f.z).normalize();
    f_flattened
}

fn strafe_vector(rotation: &Quat) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Quat::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}
fn movement_axis(input: &Res<Input<KeyCode>>, plus: KeyCode, minus: KeyCode) -> f32 {
    let mut axis = 0.0;
    if input.pressed(plus) {
        axis += 1.0;
    }
    if input.pressed(minus) {
        axis -= 1.0;
    }
    axis
}

fn camera_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    for (mut options, mut transform) in query.iter_mut() {
        let (axis_h, axis_v, axis_float) = if options.enabled {
            (
                movement_axis(&keyboard_input, options.key_right, options.key_left),
                movement_axis(&keyboard_input, options.key_backward, options.key_forward),
                movement_axis(&keyboard_input, options.key_up, options.key_down),
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let rotation = transform.rotation;
        let accel: Vec3 = (strafe_vector(&rotation) * axis_h)
            + (forward_walk_vector(&rotation) * axis_v)
            + (Vec3::Y * axis_float);
        let accel: Vec3 = if accel.length() != 0.0 {
            accel.normalize() * options.accel
        } else {
            Vec3::ZERO
        };

        let friction: Vec3 = if options.velocity.length() != 0.0 {
            options.velocity.normalize() * -1.0 * options.friction
        } else {
            Vec3::ZERO
        };

        options.velocity += accel * time.delta_seconds();

        // clamp within max speed
        if options.velocity.length() > options.max_speed {
            options.velocity = options.velocity.normalize() * options.max_speed;
        }

        let delta_friction = friction * time.delta_seconds();

        options.velocity =
            if (options.velocity + delta_friction).signum() != options.velocity.signum() {
                Vec3::ZERO
            } else {
                options.velocity + delta_friction
            };

        transform.translation += options.velocity;
    }
}
