use bevy::prelude::*;
use rand::prelude::random;
use std::time::Duration;

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);

#[derive(PartialEq, Clone, Copy)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Resource, Default)]
struct SnakeSegments(Vec<Entity>);

#[derive(Resource, Default)]
struct LastTailPosition(Option<Position>);

#[derive(Component)]
struct Food;

#[derive(Resource)]
struct FixedTimer(Timer);

#[derive(Resource)]
struct FoodSpawnerTimer(Timer);

#[derive(Event)]
struct GrowthEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Snake!".to_string(),
                resolution: (500.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
        .insert_resource(FixedTimer(Timer::new(
            Duration::from_millis(250),
            TimerMode::Repeating,
        )))
        .insert_resource(FoodSpawnerTimer(Timer::new(
            Duration::from_secs(1),
            TimerMode::Repeating,
        )))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_systems(Startup, (setup_camera, spawn_snake))
        .add_systems(
            Update,
            (
                (
                    snake_movement_input,
                    snake_movement,
                    snake_eating,
                    snake_growth,
                )
                    .chain(),
                food_spawner,
            ),
        )
        .add_systems(PostUpdate, (position_translation, size_scaling))
        .add_event::<GrowthEvent>()
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn size_scaling(window: Single<&Window>, mut q: Query<(&Size, &mut Transform)>) {
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        )
    }
}

fn position_translation(window: Single<&Window>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }

    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        )
    }
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            .spawn((
                Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                Transform {
                    scale: Vec3::new(10.0, 10.0, 10.0),
                    ..default()
                },
            ))
            .insert((
                SnakeHead {
                    direction: Direction::Up,
                },
                SnakeSegment,
                Position { x: 3, y: 3 },
                Size::square(0.8),
            ))
            .id(),
        spawn_snake_segment(commands, Position { x: 3, y: 2 }),
    ]);
}

fn spawn_snake_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn(Sprite {
            color: SNAKE_SEGMENT_COLOR,
            ..default()
        })
        .insert((SnakeSegment, position, Size::square(0.65)))
        .id()
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Direction::Right
        } else {
            head.direction
        };
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_movement(
    time: Res<Time>,
    mut timer: ResMut<FixedTimer>,
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut positions: Query<&mut Position>,
    mut last_tail_position: ResMut<LastTailPosition>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    if let Some((head_entity, head)) = heads.iter_mut().next() {
        // get position for every snake segment
        let segment_positions: Vec<Position> = segments
            .0
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect();

        // get position for the snake's head
        let mut head_pos = positions.get_mut(head_entity).unwrap();

        // proceed with moving the snake's head
        match &head.direction {
            Direction::Left => {
                head_pos.x -= 1;
            }
            Direction::Right => {
                head_pos.x += 1;
            }
            Direction::Up => {
                head_pos.y += 1;
            }
            Direction::Down => {
                head_pos.y -= 1;
            }
        };

        // segment_position = n, segment = n + 1
        // I.e. for each segment position, we have access to the next segment
        // set the position of the next segment to the current position
        segment_positions
            .iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });

        // update last tail position with last segment position
        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
    }
}

fn food_spawner(time: Res<Time>, mut timer: ResMut<FoodSpawnerTimer>, mut commands: Commands) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    commands
        .spawn(Sprite {
            color: FOOD_COLOR,
            ..default()
        })
        .insert((
            Food,
            Position {
                x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
                y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
            },
            Size::square(0.8),
        ));
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        segments
            .0
            .push(spawn_snake_segment(commands, last_tail_position.0.unwrap()));
    }
}
