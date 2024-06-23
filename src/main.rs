use rand::{thread_rng, Rng};

use bevy::{
    math::bounding::{
        Aabb2d,
        BoundingCircle,
        IntersectsVolume,
    },
    prelude::*,
    window::WindowResolution,
};

const ARENA_HEIGHT: i32 = 20;
const ARENA_WIDTH: i32 = 20;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const MONGOOSE_SIZE: Vec2 = Vec2::new(20.0, 20.0);

const BERRY_DIAMETER: f32 = 15.0;

const BACKGROUND_COLOR: Color = Color::rgb(0.6, 0.9, 0.2);
const MONGOOSE_COLOR: Color = Color::rgb(0.8, 0.6, 0.0);
const BERRY_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct MovementTimer(Timer);

#[derive(Component)]
struct MongooseHead;

#[derive(Component)]
struct MongooseBody;

#[derive(Component)]
struct SnakeHead;

#[derive(Component)]
struct SnakeSegment;

#[derive(Component)]
struct Snake;

#[derive(Component)]
struct Berry;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Event, Default)]
struct CollisionEvent;

// This resource tracks the game's score
#[derive(Resource)]
struct Scoreboard { score: usize }

#[derive(Component)]
struct ScoreboardUI;

#[derive(Resource)]
struct BerrySpawnTimer(Timer);

fn spawn_mongoose(commands: &mut Commands) {
    let (x, y) = (0, 0);
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                scale: MONGOOSE_SIZE.extend(1.0),
                ..default()
            },
            sprite: Sprite {
                color: MONGOOSE_COLOR,
                ..default()
            },
            ..default()
        },
        MongooseHead,
        Collider,
        Position { x, y },
        MovementTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
    ));
}

fn spawn_scoreboard(commands: &mut Commands) {
    commands.spawn((
        ScoreboardUI,
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: TEXT_COLOR,
                    ..default()
                },
            ),
            TextSection::from_style(
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: SCORE_COLOR,
                    ..default()
                }
            ),
        ]).with_style(Style {
            position_type: PositionType::Absolute,
            top: SCOREBOARD_TEXT_PADDING,
            left: SCOREBOARD_TEXT_PADDING,
            ..default()
        }),
    ));
}

fn spawn_berry(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<BerrySpawnTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = thread_rng();
        let x = rng.gen_range(0..ARENA_WIDTH);
        let y = rng.gen_range(0..ARENA_HEIGHT);
        commands.spawn((
            SpriteBundle {
                transform: Transform {
                    scale: Vec2::splat(BERRY_DIAMETER).extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: BERRY_COLOR,
                    ..default()
                },
                ..default()
            },
            Berry,
            Collider,
            Position { x, y }
        ));
    }
}

fn spawn_snake(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("snake.png");
    let texture_atlas_layout = texture_atlas_layouts.add(
        TextureAtlasLayout::from_grid(Vec2::splat(40.0), 3, 1, None, None)
    );
    let x = 5;
    let y = 5;
    let snake = commands.spawn((
        SpriteBundle::default(),
        Snake,
        MovementTimer(Timer::from_seconds(1.0, TimerMode::Repeating)),
    )).id();
    let head = commands.spawn((
        SpriteBundle {
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 0,
        },
        SnakeHead,
        SnakeSegment,
        Collider,
        Position { x, y },
    )).id();
    commands.entity(snake).add_child(head);
    let body = commands.spawn((
        SpriteBundle {
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 1,
        },
        SnakeSegment,
        Collider,
        Position { x: x+1, y },
    )).id();
    commands.entity(snake).add_child(body);
    let tail = commands.spawn((
        SpriteBundle {
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 2,
        },
        SnakeSegment,
        Collider,
        Position { x: x+2, y },
    )).id();
    commands.entity(snake).add_child(tail);
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_mongoose(&mut commands);
    spawn_scoreboard(&mut commands);
    spawn_snake(commands, asset_server, texture_atlas_layouts);
}

fn mongoose_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut position_query: Query<&mut Position, With<MongooseHead>>,
    mut timer: Query<&mut MovementTimer, With<MongooseHead>>,
    time: Res<Time>,
) {
    let mut timer = timer.get_single_mut().unwrap();
    if timer.0.tick(time.delta()).just_finished() {
        let mut position = position_query.single_mut();

        let mut delta_x = 0;
        let mut delta_y = 0;

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            delta_x -= 1;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            delta_x += 1;
        }
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            delta_y += 1;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            delta_y -= 1;
        }
        position.x += delta_x;
        position.y += delta_y;
    }
}

fn move_snakes(
    mut snakes_query: Query<(Entity, &Children, &mut MovementTimer), With<Snake>>,
    mut positions_query: Query<&mut Position, With<SnakeSegment>>,
    time: Res<Time>,
) {
    for (_, segments_entities, mut timer) in &mut snakes_query {
        if timer.0.tick(time.delta()).just_finished() {
            let delta_x = -1; // FIXME
            let delta_y = 0;
            let head = segments_entities.get(0).unwrap();
            let mut head_position = positions_query.get_mut(*head).unwrap();
            head_position.x += delta_x;
            head_position.y += delta_y;
            // Check for turns and swap deltas as needed
            for segment in segments_entities.get(1..).unwrap().iter() {
                let mut segment_position = positions_query.get_mut(*segment).unwrap();
                segment_position.x += delta_x;
                segment_position.y += delta_y;
            }
        }
    }
}

fn transformation(window: Query<&Window>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = window.single();
    for (pos, mut transform) in &mut q {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn berry_collision(
    berry: BoundingCircle,
    bounding_box: Aabb2d,
) -> bool {
    berry.intersects(&bounding_box)
}

fn check_for_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut mongoose_query: Query<&Transform, With<MongooseHead>>,
    collider_query: Query<(Entity, &Transform, Option<&Berry>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let mongoose_transform= mongoose_query.single_mut();

    for (collider_entity, collider_transform, maybe_berry) in &collider_query {
        let collision = berry_collision(
            BoundingCircle::new(mongoose_transform.translation.truncate(), BERRY_DIAMETER/2.0),
            Aabb2d::new(
                collider_transform.translation.truncate(),
                collider_transform.scale.truncate()/2.0,
            ),
        );

        if collision {
            // Sends a collision event so that other systems can react to the collision
            collision_events.send_default();

            // Berries should be despawned and increment the scoreboard on collision
            if maybe_berry.is_some() {
                commands.entity(collider_entity).despawn();
                scoreboard.score += 1;
            }
        }
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text, With<ScoreboardUI>>) {
    let mut text = query.single_mut();
    text.sections[1].value = scoreboard.score.to_string();
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mongoose!".into(),
                resolution: WindowResolution::new(800., 800.).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(BerrySpawnTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
        .add_event::<CollisionEvent>()
        .add_systems(Startup, setup)
        // Add our gameplay simulation systems to the fixed timestep schedule
        // which runs at 64 Hz by default
        .add_systems(
            FixedUpdate,
            (
                mongoose_control,
                move_snakes,
                spawn_berry,
                transformation,
                check_for_collisions,
            ).chain(),
        )
        .add_systems(
            Update,
            (
                update_scoreboard,
                bevy::window::close_on_esc,
            )
        )
        .run();
}
