use array2d::Array2D;
use itertools::Itertools;
use rand::{thread_rng, Rng};

use bevy::{prelude::*, window::WindowResolution};

const ARENA_HEIGHT: i32 = 20;
const ARENA_WIDTH: i32 = 20;

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

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Segmented {
    head_position: Position,
    segments: Vec<Entity>,
}

#[derive(Component)]
struct Mongoose;

#[derive(Component)]
struct Snake;

#[derive(Component, Default)]
// imagine some humongous quotation marks here
struct AI {
    move_timer: Timer,
    plan_timer: Timer,
    next: Option<usize>, // LEFT, UP, RIGHT, or DOWN
    plan: Option<Target>,
}

enum Target {
    Position(Position),
    Entity(Entity),
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

#[derive(Resource)]
struct Arena(Array2D<bool>);

#[derive(Event)]
struct GrowEvent {
    segmented: Entity,
}

impl Arena {
    fn set(&mut self, x: i32, y: i32) {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return;
        }
        if self.0[(y as usize, x as usize)] {
            panic!("Setting arena location ({} {}) that was already set", x, y);
        }
        self.0[(y as usize, x as usize)] = true;
    }
    fn unset(&mut self, x: i32, y: i32) {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return;
        }
        if !self.0[(y as usize, x as usize)] {
            panic!(
                "Unsetting arena location ({} {}) that was already unset",
                x, y
            );
        }
        self.0[(y as usize, x as usize)] = false;
    }
    fn isset(&self, x: i32, y: i32) -> bool {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return false;
        }
        self.0[(y as usize, x as usize)]
    }
}
fn spawn_mongoose(
    mut commands: Commands,
    mut arena: ResMut<Arena>,
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
    let head_position = Position { x, y };
    let mut segments: Vec<Entity> = Vec::new();
    segments.push(
        commands
            .spawn((
                SpriteBundle {
                    texture: texture.clone(),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    ..default()
                },
                Position { x, y },
                Mongoose,
            ))
            .id(),
    );
    arena.set(x, y);
    segments.push(
        commands
            .spawn((
                SpriteBundle {
                    texture: texture.clone(),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    index: BODY + CCW_LEFT,
                },
                Position { x: x + 1, y },
                Mongoose,
            ))
            .id(),
    );
    arena.set(x + 1, y);
    segments.push(
        commands
            .spawn((
                SpriteBundle {
                    texture: texture.clone(),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    index: TAIL + UP,
                },
                Position { x: x + 1, y: y - 1 },
                Mongoose,
            ))
            .id(),
    );
    arena.set(x + 1, y - 1);
    commands.spawn((
        Segmented {
            head_position,
            segments,
        },
        Mongoose,
    ));
}

fn spawn_scoreboard(mut commands: Commands) {
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

fn spawn_snakes(
    mut commands: Commands,
    mut arena: ResMut<Arena>,
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
    let n = rng.gen_range(0..=3); // number of starting body segments
    let (mut x, mut y, delta_x, delta_y) = loop {
        let p = rng.gen_range(-2..ARENA_HEIGHT + 2); // TODO: check distribution of extant snakes to balance spawn locations
        let side = rng.gen_range(0..4);
        let (x, y, delta_x, delta_y) = match side {
            LEFT => (-1, p, -1, 0),
            UP => (p, 20, 0, 1),
            RIGHT => (20, p, 1, 0),
            DOWN => (p, -1, 0, -1),
            _ => panic!("Bad spawn side"),
        };
        if !arena.isset(x, y) {
            break (x, y, delta_x, delta_y);
        }
    };
    let mut segments: Vec<Entity> = Vec::new();
    segments.push(
        commands
            .spawn((
                SpriteBundle {
                    texture: texture.clone(),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    ..default()
                },
                Position { x, y },
                Snake,
            ))
            .id(),
    );
    arena.set(x, y);
    for _ in 1..=n {
        x += delta_x;
        y += delta_y;
        segments.push(
            commands
                .spawn((
                    SpriteBundle {
                        texture: texture.clone(),
                        ..default()
                    },
                    TextureAtlas {
                        layout: texture_atlas_layout.clone(),
                        ..default()
                    },
                    Position { x, y },
                    Snake,
                ))
                .id(),
        );
        arena.set(x, y);
    }
    x += delta_x;
    y += delta_y;
    segments.push(
        commands
            .spawn((
                SpriteBundle {
                    texture: texture.clone(),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    ..default()
                },
                Position { x, y },
                Snake,
            ))
            .id(),
    );
    arena.set(x, y);

    commands.spawn((
        AI {
            move_timer: Timer::from_seconds(SNAKE_MOVEMENT_PERIOD, TimerMode::Once),
            plan_timer: Timer::from_seconds(SNAKE_PLANNING_PERIOD, TimerMode::Once),
            ..default()
        },
        Segmented {
            head_position: Position { x, y },
            segments,
        },
        Snake,
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn mongoose_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mongoose: Query<&mut Segmented, With<Mongoose>>,
    mut positions: Query<&mut Position, With<Mongoose>>,
    mut arena: ResMut<Arena>,
    mut input_timer: ResMut<InputTimer>,
    time: Res<Time>,
) {
    // TODO move this into a keyboard_input system
    // This system will take events instead
    if !input_timer.0.tick(time.delta()).finished() {
        return;
    }

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

    let next_direction = if delta_x < 0 {
        LEFT
    } else if delta_y > 0 {
        UP
    } else if delta_x > 0 {
        RIGHT
    } else if delta_y < 0 {
        DOWN
    } else {
        panic!();
    };

    let mut mongoose = mongoose.get_single_mut().expect("Mongoose entity missing");

    if mongoose.head_position.x == 0 && next_direction == LEFT {
        return;
    }
    if mongoose.head_position.y == ARENA_HEIGHT - 1 && next_direction == UP {
        return;
    }
    if mongoose.head_position.x == ARENA_WIDTH - 1 && next_direction == RIGHT {
        return;
    }
    if mongoose.head_position.y == 0 && next_direction == DOWN {
        return;
    }
    if arena.isset(
        mongoose.head_position.x + delta_x,
        mongoose.head_position.y + delta_y,
    ) {
        // Space is occupied
        return;
    }
    mongoose.head_position.x += delta_x;
    mongoose.head_position.y += delta_y;
    arena.set(mongoose.head_position.x, mongoose.head_position.y);
    let mut gap_position = mongoose.head_position.clone();
    for s in mongoose.segments.iter() {
        let mut position = positions.get_mut(*s).unwrap();
        (position.x, gap_position.x) = (gap_position.x, position.x);
        (position.y, gap_position.y) = (gap_position.y, position.y);
    }
    arena.unset(gap_position.x, gap_position.y);
    input_timer.0.reset();
}

fn snakes_movement(
    mut snakes: Query<(&mut AI, &mut Segmented), With<Snake>>,
    mut positions: Query<&mut Position, With<Snake>>,
    mut arena: ResMut<Arena>,
    time: Res<Time>,
) {
    for (mut ai, mut snake) in &mut snakes {
        if !ai.move_timer.tick(time.delta()).finished() {
            continue;
        }
        let mut delta_x = 0;
        let mut delta_y = 0;
        if let Some(next_direction) = ai.next {
            if next_direction == LEFT {
                delta_x = -1;
            } else if next_direction == RIGHT {
                delta_x = 1;
            } else if next_direction == UP {
                delta_y = 1;
            } else if next_direction == DOWN {
                delta_y = -1;
            }
            if arena.isset(
                snake.head_position.x + delta_x,
                snake.head_position.y + delta_y,
            ) {
                // Space is occupied or outside the arena
                return;
            }
            snake.head_position.x += delta_x;
            snake.head_position.y += delta_y;
            arena.set(snake.head_position.x, snake.head_position.y);
            let mut gap_position = snake.head_position.clone();
            let mut grown = false;
            for s in snake.segments.iter() {
                let mut position = positions.get_mut(*s).unwrap();
                (position.x, gap_position.x) = (gap_position.x, position.x);
                (position.y, gap_position.y) = (gap_position.y, position.y);
                if position.x == gap_position.x && position.y == gap_position.y {
                    grown = true;
                }
            }
            if !grown {
                arena.unset(gap_position.x, gap_position.y);
            }
        }
        ai.move_timer.reset();
    }
}

fn snakes_planning(
    query: Query<(Entity, &Position), With<Berry>>,
    mut snakes: Query<(&mut AI, &Segmented), With<Snake>>,
    time: Res<Time>,
) {
    for (mut ai, snake) in &mut snakes {
        if !ai.plan_timer.tick(time.delta()).finished() {
            continue;
        }
        // TODO just take the first berry for now
        if let Some((berry, _)) = query.iter().next() {
            ai.plan = Some(Target::Entity(berry));
        }

        if let Some(goal) = match &ai.plan {
            Some(Target::Entity(entity)) => match query.get(*entity) {
                Ok((_, position)) => Some(position),
                Err(_) => None,
            },
            Some(Target::Position(position)) => Some(position),
            None => None,
        } {
            // pathfinding o_o
            let delta_x = goal.x - snake.head_position.x;
            let delta_y = goal.y - snake.head_position.y;
            if delta_x < 0 {
                ai.next = Some(LEFT);
            } else if delta_x > 0 {
                ai.next = Some(RIGHT);
            } else if delta_y < 0 {
                ai.next = Some(DOWN);
            } else if delta_y > 0 {
                ai.next = Some(UP);
            } else {
                // Wherever you go, there you are
                ai.next = None;
                ai.plan = None;
                ai.plan_timer.reset();
            }
        } else {
            // Target disappeared
            ai.next = None;
            ai.plan = None;
            ai.plan_timer.reset();
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

fn set_segment_sprites(
    things: Query<&Segmented>,
    mut segments: Query<(&Position, &mut TextureAtlas)>,
) {
    'things: for thing in &things {
        // TODO do this only after movement, maybe check for a needs_redraw flag
        let i_tail = thing.segments.len() - 2;
        for (i, (f, b)) in thing.segments.iter().tuple_windows().enumerate() {
            let [(pos_f, mut ta_f), (pos_b, mut ta_b)] = segments
                .get_many_mut([*f, *b])
                .expect("Failed to get segments pair");

            let direction = if pos_f.x - pos_b.x == -1 {
                Some(LEFT)
            } else if pos_f.x - pos_b.x == 1 {
                Some(RIGHT)
            } else if pos_f.y - pos_b.y == -1 {
                Some(DOWN)
            } else if pos_f.y - pos_b.y == 1 {
                Some(UP)
            } else if pos_f.x == pos_b.x && pos_f.y == pos_b.y {
                None // Growth just occured
            } else {
                panic!("Successive segments are neither adjacent nor at the same place");
            };
            if direction == None {
                ta_f.index += TAIL;
                ta_b.index = SPRITE_SHEET_COLUMNS - 1; // Should be a blank sprite
                continue 'things;
            }
            if i == 0 {
                // Entity f is the head segment
                ta_f.index = HEAD + direction.unwrap();
            } else {
                ta_f.index = BODY
                    + match (direction.unwrap(), ta_f.index) {
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
                        _ => 0, // FIXME restore the panic after we've implemented segments collision detection
                                /*_ => panic!(
                                    "Nonsense pair of directions {} {}",
                                    direction.unwrap(),
                                    ta_f.index
                                ),*/
                    };
            }
            if i == i_tail {
                // Entity b is the tail segment
                ta_b.index = TAIL + direction.unwrap();
            } else {
                ta_b.index = direction.unwrap();
            }
        }
    }
}

fn eat_berries(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mongoose: Query<&Segmented, With<Mongoose>>,
    snakes: Query<(Entity, &Segmented), With<Snake>>,
    query: Query<(Entity, &Position), With<Berry>>,
    mut writer: EventWriter<GrowEvent>,
) {
    let mongoose_position = mongoose
        .get_single()
        .expect("Mongoose entity missing")
        .head_position;

    for (berry, berry_position) in &query {
        if mongoose_position == *berry_position {
            commands.entity(berry).despawn();
            scoreboard.berries_eaten_by_mongoose += 1;
        } else {
            for (entity, snake) in &snakes {
                if snake.head_position == *berry_position {
                    commands.entity(berry).despawn();
                    scoreboard.berries_eaten_by_snakes += 1;
                    writer.send(GrowEvent { segmented: entity });
                }
            }
        }
    }
}

fn grow_snakes(
    mut commands: Commands,
    mut snakes: Query<&mut Segmented>,
    positions: Query<&Position>,
    mut reader: EventReader<GrowEvent>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for event in reader.read() {
        if let Ok(mut segmented) = snakes.get_mut(event.segmented) {
            let texture = asset_server.load("snake.png");
            let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
                Vec2::splat(40.0),
                SPRITE_SHEET_COLUMNS,
                SPRITE_SHEET_ROWS,
                None,
                None,
            ));
            let tail_position = positions
                .get(*segmented.segments.last().expect("Segments vector is empty"))
                .expect("Tail position missing")
                .clone();
            segmented.segments.push(
                commands
                    .spawn((
                        SpriteBundle {
                            texture: texture.clone(),
                            ..default()
                        },
                        TextureAtlas {
                            layout: texture_atlas_layout.clone(),
                            ..default()
                        },
                        tail_position,
                        Snake,
                    ))
                    .id(),
            )
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
        .add_event::<GrowEvent>()
        .insert_resource(Arena(Array2D::filled_with(
            false,
            ARENA_HEIGHT as usize,
            ARENA_WIDTH as usize,
        )))
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
        .add_systems(Startup, (setup, spawn_scoreboard, spawn_mongoose).chain())
        // Add our gameplay simulation systems to the fixed timestep schedule
        // which runs at 64 Hz by default
        .add_systems(
            FixedUpdate,
            (
                spawn_snakes,
                snakes_planning,
                snakes_movement,
                mongoose_movement,
                eat_berries,
                grow_snakes,
                set_segment_sprites,
                spawn_berry,
                transformation,
            )
                .chain(),
        )
        .add_systems(Update, (update_scoreboard, bevy::window::close_on_esc))
        .run();
}
