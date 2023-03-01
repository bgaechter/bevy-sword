use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
pub use bevy_inspector_egui::quick::WorldInspectorPlugin;
pub use leafwing_input_manager::prelude::*;
pub use leafwing_input_manager::{errors::NearlySingularConversion, orientation::Direction};
pub use bevy_mod_picking::*;


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

#[derive(Component)]
struct Player;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum ArpgAction {
    // Movement
    Up,
    Down,
    Left,
    Right,
    // Abilities
    Ability1,
    Ability2,
    Ability3,
    Ability4,
    Ultimate,
}

impl ArpgAction {
    // Lists like this can be very useful for quickly matching subsets of actions
    const DIRECTIONS: [Self; 4] = [
        ArpgAction::Up,
        ArpgAction::Down,
        ArpgAction::Left,
        ArpgAction::Right,
    ];

    fn direction(self) -> Option<Direction> {
        match self {
            ArpgAction::Up => Some(Direction::NORTH),
            ArpgAction::Down => Some(Direction::SOUTH),
            ArpgAction::Left => Some(Direction::WEST),
            ArpgAction::Right => Some(Direction::EAST),
            _ => None,
        }
    }
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    // This bundle must be added to your player entity
    // (or whatever else you wish to control)
    #[bundle]
    input_manager: InputManagerBundle<ArpgAction>,
}
impl PlayerBundle {
    fn default_input_map() -> InputMap<ArpgAction> {
        // This allows us to replace `ArpgAction::Up` with `Up`,
        // significantly reducing boilerplate
        use ArpgAction::*;
        let mut input_map = InputMap::default();

        // Movement
        input_map.insert(KeyCode::Up, Up);
        input_map.insert(GamepadButtonType::DPadUp, Up);

        input_map.insert(KeyCode::Down, Down);
        input_map.insert(GamepadButtonType::DPadDown, Down);

        input_map.insert(KeyCode::Left, Left);
        input_map.insert(GamepadButtonType::DPadLeft, Left);

        input_map.insert(KeyCode::Right, Right);
        input_map.insert(GamepadButtonType::DPadRight, Right);

        // Abilities
        input_map.insert(KeyCode::Q, Ability1);
        input_map.insert(GamepadButtonType::West, Ability1);
        input_map.insert(MouseButton::Left, Ability1);

        input_map.insert(KeyCode::W, Ability2);
        input_map.insert(GamepadButtonType::North, Ability2);
        input_map.insert(MouseButton::Right, Ability2);

        input_map.insert(KeyCode::E, Ability3);
        input_map.insert(GamepadButtonType::East, Ability3);

        input_map.insert(KeyCode::Space, Ability4);
        input_map.insert(GamepadButtonType::South, Ability4);

        input_map.insert(KeyCode::R, Ultimate);
        input_map.insert(GamepadButtonType::LeftTrigger2, Ultimate);

        input_map
    }
}

#[derive(Default, Resource)]
struct Game {
    map: Map,
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
    App::new()
        .init_resource::<Game>()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputManagerPlugin::<ArpgAction>::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin)
        // Mod Picking
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(DebugCursorPickingPlugin) // <- Adds the debug cursor (optional)
        .add_plugin(DebugEventsPickingPlugin)
        .add_state(GameState::Playing)
        .add_startup_system(setup_cameras)
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                //.with_system(move_player)
                // .with_system(camera_movement_system),
                // .with_system(movement)
                // .with_system(player_input)
        )
        .add_startup_system(spawn_player)
        // The ActionState can be used directly
        .add_system(cast_fireball)
        // Or multiple parts of it can be inspected
        .add_system(player_dash)
        // Or it can be used to emit events for later processing
        .add_event::<PlayerWalk>()
        .add_system(player_walks)
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(teardown))
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn setup_cameras(mut commands: Commands, mut game: ResMut<Game>) {
    game.camera_should_focus = Vec3::from(RESET_FOCUS);
    game.camera_is_focus = game.camera_should_focus;
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(
                -(MAP_SIZE_WIDTH as f32 / 2.0),
                2.0 * MAP_SIZE_HEIGHT as f32 / 3.0,
                MAP_SIZE_HEIGHT as f32 / 2.0 - 0.5,
        )
        .looking_at(game.camera_is_focus, Vec3::Y),
        ..default()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut game: ResMut<Game>) {
    let mut rng = RandomNumberGenerator::new();
    let map_builder = MapBuilder::new(&mut rng);
    game.score = 0;

    game.map = map_builder.map;

    commands.spawn(PointLightBundle {
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
                commands.spawn(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        0.2,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                }).insert(PickableBundle::default());
            }
            TileType::Floor => {
                commands.spawn(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        0.,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                }).insert(PickableBundle::default());
            }
            TileType::Exit => {
                commands.spawn(SceneBundle {
                    transform: Transform::from_xyz(
                        game.map.index_to_point2d(idx).x as f32,
                        -0.2,
                        game.map.index_to_point2d(idx).y as f32,
                    ),
                    scene: cell_scene.clone(),
                    ..default()
                }).insert(PickableBundle::default());
            }
        });

    // scoreboard
    commands.spawn(
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


fn spawn_player(mut commands: Commands,asset_server: Res<AssetServer>) {
    commands.spawn(PlayerBundle {
        player: Player,
        input_manager: InputManagerBundle {
            input_map: PlayerBundle::default_input_map(),
            ..default()
        },
    }).insert(SceneBundle {
        transform: Transform {
            translation: Vec3::new(5 as f32, 0., 5 as f32),
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            ..default()
        },
        scene: asset_server.load("resources/alien.glb#Scene0"),
        ..default()
    });
}

fn cast_fireball(query: Query<&ActionState<ArpgAction>, With<Player>>) {
    let action_state = query.single();

    if action_state.just_pressed(ArpgAction::Ability1) {
        println!("Fwoosh!");
    }
}

fn player_dash(query: Query<&ActionState<ArpgAction>, With<Player>>) {
    let action_state = query.single();

    if action_state.just_pressed(ArpgAction::Ability4) {
        let mut direction_vector = Vec2::ZERO;

        for input_direction in ArpgAction::DIRECTIONS {
            if action_state.pressed(input_direction) {
                if let Some(direction) = input_direction.direction() {
                    // Sum the directions as 2D vectors
                    direction_vector += Vec2::from(direction);
                }
            }
        }

        // Then reconvert at the end, normalizing the magnitude
        let net_direction: Result<Direction, NearlySingularConversion> =
            direction_vector.try_into();

        if let Ok(direction) = net_direction {
            println!("Dashing in {direction:?}");
        }
    }
}

pub struct PlayerWalk {
    pub direction: Direction,
}

fn player_walks(
    query: Query<&ActionState<ArpgAction>, With<Player>>,
    mut event_writer: EventWriter<PlayerWalk>,
    mut player_query: Query<&mut Transform, With<Player>>
) {
    let action_state = query.single();
    let mut player = player_query.single_mut();

    let mut direction_vector = Vec2::ZERO;

    for input_direction in ArpgAction::DIRECTIONS {
        if action_state.pressed(input_direction) {
            if let Some(direction) = input_direction.direction() {
                // Sum the directions as 2D vectors
                direction_vector += Vec2::from(direction);
            }
        }
    }

    // Then reconvert at the end, normalizing the magnitude
    let net_direction: Result<Direction, NearlySingularConversion> = direction_vector.try_into();

    if let Ok(direction) = net_direction {
        player.translation += Vec3::new(direction.unit_vector().y, 0.0, direction.unit_vector().x);
        println!("Player walks x:{} y:{}", direction.unit_vector().x, direction.unit_vector().y);
        event_writer.send(PlayerWalk { direction });
    }
}