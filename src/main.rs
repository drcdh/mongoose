use rand::{thread_rng, Rng};

use bevy::{prelude::*, window::WindowResolution};

const ARENA_HEIGHT: i32 = 20;
const ARENA_WIDTH: i32 = 20;

/* TODO
const BERRY_Z: i32 = 1;
const SNAKE_Z: i32 = 5;
const MONGOOSE_Z: i32 = 10;
 */
const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BERRY_DIAMETER: f32 = 15.0;

const BACKGROUND_COLOR: Color = Color::rgb(0.6, 0.9, 0.2);
const BERRY_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

const SPRITE_SHEET_COLUMNS: usize = 12;
const SPRITE_SHEET_ROWS: usize = 3;

const HEAD: usize = 0;
const BODY: usize = 1 * SPRITE_SHEET_COLUMNS;
const TAIL: usize = 2 * SPRITE_SHEET_COLUMNS;

const LEFT: usize = 0;
const UP: usize = 1;
const RIGHT: usize = 2;
const DOWN: usize = 3;
const CW_LEFT: usize = 4;
const CW_UP: usize = 5;
const CW_RIGHT: usize = 6;
const CW_DOWN: usize = 7;
const CCW_LEFT: usize = 8;
const CCW_UP: usize = 9;
const CCW_RIGHT: usize = 10;
const CCW_DOWN: usize = 11;

const SNAKE_MOVEMENT_PERIOD: f32 = 0.5; // How often snakes move
const SNAKE_PLANNING_PERIOD: f32 = 3.0; // How often snakes replan their goal position

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct MovementTimer(Timer);

#[derive(Component)]
struct PlanningTimer(Timer);

#[derive(Component)]
struct MongooseHead;

#[derive(Component)]
struct MongooseBody;

#[derive(Component)]
struct MongooseSegment {
    from: usize,
    to: usize,
    type_offset: usize, // HEAD, BODY, or TAIL
}

#[derive(Component)]
struct Mongoose;

#[derive(Component)]
struct SnakeHead;

#[derive(Component)]
struct SnakeSegment {
    from: usize,
    to: usize,
    type_offset: usize, // HEAD, BODY, or TAIL
}

#[derive(Component)]
struct Snake {
    next: Option<usize>, // LEFT, UP, RIGHT, or DOWN
}

enum Target {
    Position(Position),
    Entity(Entity),
}

#[derive(Component)]
struct Plan {
    target: Option<Target>,
}

#[derive(Component)]
struct Berry;

#[derive(Resource, Default)]
struct Scoreboard {
    berries_eaten_by_mongoose: usize,
    berries_eaten_by_snakes: usize,
    snakes_killed: usize,
    mice_eaten_by_mongoose: usize,
    mice_eaten_by_snakes: usize,
    mice_escaped: usize,
}

#[derive(Component)]
struct ScoreboardUI;

#[derive(Resource)]
struct InputTimer(Timer);

#[derive(Resource)]
struct BerrySpawnTimer(Timer);

#[derive(Resource)]
struct SnakeSpawnTimer(Timer);

fn spawn_mongoose(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("mongoose.png");
    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::splat(40.0),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    ));
    let (x, y) = (ARENA_WIDTH / 2, ARENA_HEIGHT / 2);
    let mongoose = commands.spawn((SpriteBundle::default(), Mongoose)).id();
    let head = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                ..default()
            },
            MongooseHead,
            MongooseSegment {
                to: UP,
                from: LEFT,
                type_offset: HEAD,
            },
            Position { x, y },
        ))
        .id();
    commands.entity(mongoose).add_child(head);
    let body = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: BODY + CCW_LEFT,
            },
            MongooseSegment {
                to: LEFT,
                from: UP,
                type_offset: BODY,
            },
            Position { x: x + 1, y },
        ))
        .id();
    commands.entity(mongoose).add_child(body);
    let tail = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: TAIL + UP,
            },
            MongooseSegment {
                to: UP,
                from: UP,
                type_offset: TAIL,
            },
            Position { x: x + 1, y: y - 1 },
        ))
        .id();
    commands.entity(mongoose).add_child(tail);
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
            TextSection::from_style(TextStyle {
                font_size: SCOREBOARD_FONT_SIZE,
                color: SCORE_COLOR,
                ..default()
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: SCOREBOARD_TEXT_PADDING,
            left: SCOREBOARD_TEXT_PADDING,
            ..default()
        }),
    ));
}

fn spawn_berry(mut commands: Commands, time: Res<Time>, mut timer: ResMut<BerrySpawnTimer>) {
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
            Position { x, y },
        ));
    }
}

fn spawn_snake(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut timer: ResMut<SnakeSpawnTimer>,
    time: Res<Time>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let texture = asset_server.load("snake.png");
    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::splat(40.0),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    ));
    let mut rng = thread_rng();
    let p = rng.gen_range(-2..ARENA_HEIGHT + 2);
    let side = rng.gen_range(0..4);
    let (x, y) = match side {
        LEFT => (-3, p),
        UP => (p, 23),
        RIGHT => (23, p),
        DOWN => (p, -3),
        _ => (0, 0), // error
    };
    let snake = commands
        .spawn((
            SpriteBundle::default(),
            Snake { next: None },
            Plan { target: None },
            MovementTimer(Timer::from_seconds(
                SNAKE_MOVEMENT_PERIOD,
                TimerMode::Repeating,
            )),
            PlanningTimer(Timer::from_seconds(
                SNAKE_PLANNING_PERIOD,
                TimerMode::Repeating,
            )),
        ))
        .id();
    let head = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                ..default()
            },
            SnakeHead,
            SnakeSegment {
                to: UP,
                from: LEFT,
                type_offset: HEAD,
            },
            Position { x, y },
        ))
        .id();
    commands.entity(snake).add_child(head);
    let body = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                ..default()
            },
            SnakeSegment {
                to: LEFT,
                from: UP,
                type_offset: BODY,
            },
            Position { x: x + 1, y },
        ))
        .id();
    commands.entity(snake).add_child(body);
    let tail = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: TAIL + UP,
            },
            SnakeSegment {
                to: UP,
                from: UP,
                type_offset: TAIL,
            },
            Position { x: x + 1, y: y - 1 },
        ))
        .id();
    commands.entity(snake).add_child(tail);
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_mongoose(&mut commands, asset_server, texture_atlas_layouts);
    spawn_scoreboard(&mut commands);
}

fn mongoose_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mongoose_query: Query<(&Mongoose, &Children)>,
    mut positions_query: Query<(&mut Position, &mut MongooseSegment)>,
    mut input_timer: ResMut<InputTimer>,
    time: Res<Time>,
) {
    if !input_timer.0.tick(time.delta()).finished() {
        return;
    }
    let (_, segments_entities) = mongoose_query.get_single_mut().unwrap();

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

    if delta_x != 0 && delta_y != 0 {
        // No moving diagonally
        return;
    }
    if delta_x == 0 && delta_y == 0 {
        return;
    }

    let mut next_direction = if delta_x < 0 {
        LEFT
    } else if delta_y > 0 {
        UP
    } else if delta_x > 0 {
        RIGHT
    } else if delta_y < 0 {
        DOWN
    } else {
        0 // error
    };

    // TODO repeated code in snake_moving
    let mut head = true;
    for segment_entry in segments_entities {
        let (mut segment_position, mut segment) = positions_query.get_mut(*segment_entry).unwrap();
        if head {
            // Abort if we're at the edge of the area
            if segment_position.x <= 0 && next_direction == LEFT {
                return;
            }
            if segment_position.y >= ARENA_HEIGHT && next_direction == UP {
                return;
            }
            if segment_position.x >= ARENA_WIDTH && next_direction == RIGHT {
                return;
            }
            if segment_position.y <= 0 && next_direction == DOWN {
                return;
            }
            segment.to = next_direction;
        }
        segment_position.x += match segment.to {
            LEFT => -1,
            RIGHT => 1,
            _ => 0,
        };
        segment_position.y += match segment.to {
            UP => 1,
            DOWN => -1,
            _ => 0,
        };
        // Create new bindings
        let (next, to) = (next_direction, segment.to);
        (segment.to, segment.from, next_direction) = (next, to, to);
        head = false;
    }
    // TODO (see above)

    input_timer.0.reset();
}

fn snake_moving(
    mut snakes_query: Query<(&Snake, &Children, &mut MovementTimer)>,
    mut positions_query: Query<(&mut Position, &mut SnakeSegment)>,
    time: Res<Time>,
) {
    for (snake, segments_entities, mut timer) in &mut snakes_query {
        if timer.0.tick(time.delta()).just_finished() {
            if let Some(mut next_direction) = snake.next {
                let mut head = true;
                for segment_entry in segments_entities {
                    let (mut segment_position, mut segment) =
                        positions_query.get_mut(*segment_entry).unwrap();
                    if head {
                        segment.to = next_direction;
                    }
                    segment_position.x += match segment.to {
                        LEFT => -1,
                        RIGHT => 1,
                        _ => 0,
                    };
                    segment_position.y += match segment.to {
                        UP => 1,
                        DOWN => -1,
                        _ => 0,
                    };
                    // Create new bindings
                    let (next, to) = (next_direction, segment.to);
                    (segment.to, segment.from, next_direction) = (next, to, to);
                    head = false;
                }
            }
        }
    }
}

fn snake_planning(
    positions: Query<&Position>,
    berries: Query<Entity, With<Berry>>,
    mut snakes: Query<(&mut Snake, &Children, &mut Plan, &mut PlanningTimer)>,
    head_positions: Query<&Position, With<SnakeHead>>,
    time: Res<Time>,
) {
    for (mut snake, children, mut plan, mut timer) in &mut snakes {
        if timer.0.tick(time.delta()).just_finished() {
            // TODO just take the first berry for now
            if let Some(berry) = berries.iter().next() {
                plan.target = Some(Target::Entity(berry));
            }
        }
        if let Some(goal) = match &plan.target {
            Some(Target::Entity(entity)) => match positions.get(*entity) {
                Ok(position) => Some(position),
                Err(_) => None,
            },
            Some(Target::Position(position)) => Some(position),
            None => None,
        } {
            // pathfinding o_o
            let head_position = head_positions.get(*children.get(0).unwrap()).unwrap();
            let delta_x = goal.x - head_position.x;
            let delta_y = goal.y - head_position.y;
            if delta_x < 0 {
                snake.next = Some(LEFT);
            } else if delta_x > 0 {
                snake.next = Some(RIGHT);
            } else if delta_y < 0 {
                snake.next = Some(DOWN);
            } else if delta_y > 0 {
                snake.next = Some(UP);
            } else {
                // Wherever you go, there you are
                snake.next = None;
                plan.target = None;
            }
        } else {
            snake.next = None;
            plan.target = None;
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

fn set_mongoose_sprites(mut texture_atlas_query: Query<(&mut TextureAtlas, &MongooseSegment)>) {
    for (mut ta, segment) in &mut texture_atlas_query {
        ta.index = segment.type_offset;
        ta.index += match segment.type_offset {
            HEAD => match segment.from {
                LEFT => 0,
                UP => 1,
                RIGHT => 2,
                DOWN => 3,
                _ => 0, // FIXME should cause error
            },
            BODY => match (segment.from, segment.to) {
                (LEFT, LEFT) => LEFT,
                (UP, UP) => UP,
                (RIGHT, RIGHT) => RIGHT,
                (DOWN, DOWN) => DOWN,
                (DOWN, LEFT) => CW_LEFT,
                (LEFT, UP) => CW_UP,
                (UP, RIGHT) => CW_RIGHT,
                (RIGHT, DOWN) => CW_DOWN,
                (UP, LEFT) => CCW_LEFT,
                (RIGHT, UP) => CCW_UP,
                (DOWN, RIGHT) => CCW_RIGHT,
                (LEFT, DOWN) => CCW_DOWN,
                _ => 0, // FIXME should cause error
            },
            TAIL => match segment.to {
                LEFT => 0,
                UP => 1,
                RIGHT => 2,
                DOWN => 3,
                _ => 0, // FIXME should cause error
            },
            _ => 0, // FIXME should cause error
        }
    }
}

fn set_snake_sprites(mut texture_atlas_query: Query<(&mut TextureAtlas, &SnakeSegment)>) {
    for (mut ta, segment) in &mut texture_atlas_query {
        ta.index = segment.type_offset;
        ta.index += match segment.type_offset {
            HEAD => match segment.from {
                LEFT => 0,
                UP => 1,
                RIGHT => 2,
                DOWN => 3,
                _ => 0, // FIXME should cause error
            },
            BODY => match (segment.from, segment.to) {
                (LEFT, LEFT) => LEFT,
                (UP, UP) => UP,
                (RIGHT, RIGHT) => RIGHT,
                (DOWN, DOWN) => DOWN,
                (DOWN, LEFT) => CW_LEFT,
                (LEFT, UP) => CW_UP,
                (UP, RIGHT) => CW_RIGHT,
                (RIGHT, DOWN) => CW_DOWN,
                (UP, LEFT) => CCW_LEFT,
                (RIGHT, UP) => CCW_UP,
                (DOWN, RIGHT) => CCW_RIGHT,
                (LEFT, DOWN) => CCW_DOWN,
                _ => 0, // FIXME should cause error
            },
            TAIL => match segment.to {
                LEFT => 0,
                UP => 1,
                RIGHT => 2,
                DOWN => 3,
                _ => 0, // FIXME should cause error
            },
            _ => 0, // FIXME should cause error
        }
    }
}

fn eat_berries(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mongoose_position: Query<&Position, With<MongooseHead>>,
    snake_positions: Query<&Position, With<SnakeHead>>,
    berry_positions: Query<(Entity, &Position), With<Berry>>,
) {
    let mongoose_position = mongoose_position.single();

    for (berry, berry_position) in &berry_positions {
        if mongoose_position == berry_position {
            commands.entity(berry).despawn();
            scoreboard.berries_eaten_by_mongoose += 1;
        } else {
            for snake_position in &snake_positions {
                if snake_position == berry_position {
                    commands.entity(berry).despawn();
                    scoreboard.berries_eaten_by_snakes += 1;
                }
            }
        }
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text, With<ScoreboardUI>>) {
    let mut text = query.single_mut();
    text.sections[1].value = scoreboard.berries_eaten_by_mongoose.to_string();
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
        .insert_resource(Scoreboard { ..default() })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(InputTimer(Timer::from_seconds(0.2, TimerMode::Once)))
        .insert_resource(BerrySpawnTimer(Timer::from_seconds(
            3.0,
            TimerMode::Repeating,
        )))
        .insert_resource(SnakeSpawnTimer(Timer::from_seconds(
            5.0,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup)
        // Add our gameplay simulation systems to the fixed timestep schedule
        // which runs at 64 Hz by default
        .add_systems(
            FixedUpdate,
            (
                mongoose_control,
                spawn_snake,
                snake_moving,
                snake_planning,
                spawn_berry,
                transformation,
                set_mongoose_sprites,
                set_snake_sprites,
                eat_berries,
            )
                .chain(),
        )
        .add_systems(Update, (update_scoreboard, bevy::window::close_on_esc))
        .run();
}
