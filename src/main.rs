use bimap::BiMap;
use std::cmp::{max, min};
use std::collections::VecDeque;

use array2d::Array2D;
use itertools::Itertools;
use rand::{seq::IteratorRandom, thread_rng, Rng};

use bevy::{
    prelude::*,
    utils::petgraph::{
        algo::all_simple_paths, graph::NodeIndex, visit::EdgeRef, Graph, Undirected,
    },
    window::WindowResolution,
};

const ARENA_HEIGHT: i32 = 20;
const ARENA_WIDTH: i32 = 20;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.6, 0.9, 0.2);
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

const INPUT_PERIOD: f32 = 0.2;

const DEBUG_SPEEDUP: f32 = 1.0;

const BERRY_SPAWN_PERIOD: f32 = 3.0 / DEBUG_SPEEDUP;
const RAT_SPAWN_PERIOD: f32 = 5.0 / DEBUG_SPEEDUP;
const SNAKE_SPAWN_PERIOD: f32 = 5.0 / DEBUG_SPEEDUP;

const RAT_MOVEMENT_PERIOD: f32 = 0.4 / DEBUG_SPEEDUP;
const RAT_PLANNING_PERIOD: f32 = 5.0 / DEBUG_SPEEDUP;

const SNAKE_MOVEMENT_PERIOD: f32 = 0.3 / DEBUG_SPEEDUP; // How often snakes move
const SNAKE_PLANNING_PERIOD: f32 = 3.0 / DEBUG_SPEEDUP; // How often snakes replan their goal position

const RAT_BERRY_PREFERENCE: u32 = 4; // Likelihood a rat will choose to chase a berry
const RAT_WANDER_PREFERENCE: u32 = 3; // Likelihood a rat will choose to go to a random empty location

const SNAKE_RAT_PREFERENCE: u32 = 5; // Likelihood a snake will choose to chase a rat
const SNAKE_BERRY_PREFERENCE: u32 = 2; // Likelihood a snake will choose to chase a berry
const SNAKE_WANDER_PREFERENCE: u32 = 2; // Likelihood a snake will choose to go to a random empty location

const MAX_PATH_LENGTH: usize = 8; // Necessary to keep this modest, otherwise all_simple_paths takes forever

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Berry;

#[derive(Component)]
struct Mongoose;

#[derive(Component)]
struct Rat;

#[derive(Component)]
struct Snake;

#[derive(Component)]
struct Segmented {
    head_position: Position,
    segments: Vec<Entity>,
}

#[derive(Clone, Debug)]
enum Target {
    Position(Position),
    Entity(Entity),
}

#[derive(Resource, Default)]
struct Scoreboard {
    berries_eaten_by_mongoose: usize,
    berries_eaten_by_rats: usize,
    berries_eaten_by_snakes: usize,
    rats_eaten_by_mongoose: usize,
    rats_eaten_by_snakes: usize,
    _rats_escaped: usize,
    snakes_killed: usize,
}

#[derive(Component)]
struct ScoreboardUI;

#[derive(Resource)]
struct InputTimer(Timer);

#[derive(Resource)]
struct BerrySpawnTimer(Timer);

#[derive(Resource)]
struct RatSpawnTimer(Timer);

#[derive(Resource)]
struct SnakeSpawnTimer(Timer);

#[derive(Event)]
struct GrowEvent {
    segmented: Entity,
}

#[derive(Clone, Copy, Debug)]
enum Occupancy {
    Berry(Entity),
    Mongoose(Entity),
    Rat(Entity),
    Snake(Entity),
}

#[derive(Resource)]
struct Arena {
    graph: Graph<(), (), Undirected>,
    nodes: BiMap<(i32, i32), NodeIndex>,
    occ: Array2D<Option<Occupancy>>,
}
impl Arena {
    fn new() -> Arena {
        let mut graph = Graph::<(), (), Undirected>::new_undirected();
        let mut nodes = BiMap::<(i32, i32), NodeIndex>::new();
        for x in 0..ARENA_WIDTH {
            for y in 0..ARENA_HEIGHT {
                nodes.insert((x, y), graph.add_node(()));
            }
        }
        for i in 0..ARENA_WIDTH {
            for j in 0..ARENA_HEIGHT {
                if i < (ARENA_WIDTH - 1) {
                    graph.add_edge(
                        *nodes.get_by_left(&(i, j)).unwrap(),
                        *nodes.get_by_left(&(i + 1, j)).unwrap(),
                        (),
                    );
                }
                if j < (ARENA_HEIGHT - 1) {
                    graph.add_edge(
                        *nodes.get_by_left(&(i, j)).unwrap(),
                        *nodes.get_by_left(&(i, j + 1)).unwrap(),
                        (),
                    );
                }
            }
        }
        let occ = Array2D::filled_with(None, ARENA_WIDTH as usize, ARENA_HEIGHT as usize);
        Arena { graph, nodes, occ }
    }
    fn add_edges_with(&mut self, x: i32, y: i32) {
        let n = *self.nodes.get_by_left(&(x, y)).unwrap();
        if x < (ARENA_WIDTH - 1) && !self.isset(x + 1, y) {
            self.graph
                .add_edge(n, *self.nodes.get_by_left(&(x + 1, y)).unwrap(), ());
        }
        if y < (ARENA_HEIGHT - 1) && !self.isset(x, y + 1) {
            self.graph
                .add_edge(n, *self.nodes.get_by_left(&(x, y + 1)).unwrap(), ());
        }
        if x > 0 && !self.isset(x - 1, y) {
            self.graph
                .add_edge(n, *self.nodes.get_by_left(&(x - 1, y)).unwrap(), ());
        }
        if y > 0 && !self.isset(x, y - 1) {
            self.graph
                .add_edge(n, *self.nodes.get_by_left(&(x, y - 1)).unwrap(), ());
        }
    }
    fn remove_edges_with(&mut self, x: i32, y: i32) {
        let n = *self.nodes.get_by_left(&(x, y)).unwrap();
        let edges = self.graph.edges(n);
        let ids = edges.map(|er| er.id()).collect::<Vec<_>>();
        self.graph.retain_edges(|_, ei| !ids.contains(&ei))
    }
    fn set(&mut self, x: i32, y: i32, occ: Occupancy) {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return;
        }
        if self.isset(x, y) {
            panic!(
                "Setting arena location ({} {}) that was already set to {:?}",
                x,
                y,
                self.occ[(x as usize, y as usize)]
            );
        }
        self.occ[(x as usize, y as usize)] = Some(occ);
        self.remove_edges_with(x, y);
    }
    fn unset(&mut self, x: i32, y: i32) -> Option<Occupancy> {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return None;
        }
        if self.occ[(x as usize, y as usize)].is_none() {
            panic!(
                "Unsetting arena location ({} {}) that was already unset",
                x, y
            );
        }
        let occ = self.occ[(x as usize, y as usize)];
        self.occ[(x as usize, y as usize)] = None;
        self.add_edges_with(x, y);
        return occ;
    }
    fn unset_maybe(&mut self, x: i32, y: i32) -> Option<Occupancy> {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return None;
        }
        let occ = self.occ[(x as usize, y as usize)];
        self.occ[(x as usize, y as usize)] = None;
        self.add_edges_with(x, y);
        return occ;
    }
    fn isset(&self, x: i32, y: i32) -> bool {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return false;
        }
        self.occ[(x as usize, y as usize)].is_some()
    }
    fn occ(&self, x: i32, y: i32) -> Option<Occupancy> {
        if x >= ARENA_WIDTH || x < 0 || y >= ARENA_HEIGHT || y < 0 {
            // Don't bother keeping track of things offscreen, like freshly spawned snakes. Is this a good idea??
            return None;
        }
        self.occ[(x as usize, y as usize)]
    }
}

#[derive(Component, Default)]
// imagine some humongous quotation marks here
struct AI {
    move_timer: Timer,
    plan_timer: Timer,
    path: VecDeque<Position>,
    target: Option<Target>,
}
impl AI {
    fn plan_path(&mut self, p: &Position, goal: &Position, arena: &mut Arena) {
        println!("Planning to go from {:?} to {:?}", p, goal);
        // If the things occupy spaces, temporarily unset the positions for pathplanning
        let start_occ = arena.unset(p.x, p.y);
        let goal_occ = arena.unset_maybe(goal.x, goal.y);
        let paths = all_simple_paths::<Vec<_>, _>(
            &arena.graph,
            *arena.nodes.get_by_left(&(p.x, p.y)).unwrap(),
            *arena.nodes.get_by_left(&(goal.x, goal.y)).unwrap(),
            0,
            Some(MAX_PATH_LENGTH),
        )
        .collect::<Vec<_>>();

        // Undo the temporary unsets
        if start_occ.is_some() {
            arena.set(p.x, p.y, start_occ.unwrap());
        }
        if goal_occ.is_some() {
            arena.set(goal.x, goal.y, goal_occ.unwrap());
        }

        if let Some(path) = paths.first() {
            self.path = path
                .iter()
                .skip(1)
                .map(|n| {
                    let (x, y) = *arena.nodes.get_by_right(n).unwrap();
                    Position {
                        x: x as i32,
                        y: y as i32,
                    }
                })
                .collect();
        }
    }
    fn clear(&mut self) {
        self.path.clear();
        self.target = None;
    }
}

fn spawn_berries(
    mut commands: Commands,
    mut arena: ResMut<Arena>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    time: Res<Time>,
    mut timer: ResMut<BerrySpawnTimer>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let mut rng = thread_rng();
    let (x, y) = loop {
        let x = rng.gen_range(0..ARENA_WIDTH);
        let y = rng.gen_range(0..ARENA_HEIGHT);
        if !arena.isset(x, y) {
            break (x, y);
        }
    };
    let texture = asset_server.load("berry.png");
    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::splat(40.0),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    ));
    let berry = commands
        .spawn((
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                ..default()
            },
            Berry,
            Position { x, y },
        ))
        .id();
    arena.set(x, y, Occupancy::Berry(berry));
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
    let segment = commands
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
        .id();
    arena.set(x, y, Occupancy::Mongoose(segment));
    segments.push(segment);
    let segment = commands
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
        .id();
    arena.set(x + 1, y, Occupancy::Mongoose(segment));
    segments.push(segment);
    let segment = commands
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
        .id();
    arena.set(x + 1, y - 1, Occupancy::Mongoose(segment));
    segments.push(segment);
    commands.spawn((
        Segmented {
            head_position,
            segments,
        },
        Mongoose,
    ));
}

fn spawn_rats(
    mut commands: Commands,
    mut arena: ResMut<Arena>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    time: Res<Time>,
    mut timer: ResMut<RatSpawnTimer>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let mut rng = thread_rng();
    let (x, y) = loop {
        let x = rng.gen_range(0..ARENA_WIDTH);
        let y = rng.gen_range(0..ARENA_HEIGHT);
        if !arena.isset(x, y) {
            break (x, y);
        }
    };
    let texture = asset_server.load("rat.png");
    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::splat(40.0),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    ));
    let rat = commands
        .spawn((
            AI {
                move_timer: Timer::from_seconds(RAT_MOVEMENT_PERIOD, TimerMode::Once),
                plan_timer: Timer::from_seconds(RAT_PLANNING_PERIOD, TimerMode::Once),
                ..default()
            },
            SpriteBundle {
                texture: texture.clone(),
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                ..default()
            },
            Rat,
            Position { x, y },
        ))
        .id();
    arena.set(x, y, Occupancy::Rat(rat));
}

fn spawn_snakes(
    commands: Commands,
    arena: ResMut<Arena>,
    asset_server: Res<AssetServer>,
    texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut timer: ResMut<SnakeSpawnTimer>,
    time: Res<Time>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    // TODO: check distribution of extant snakes to balance spawn locations
    let mut rng = thread_rng();
    let n = rng.gen_range(0..=3); // number of starting body segments
    let (x, y, delta_x, delta_y) = loop {
        let p = rng.gen_range(0..ARENA_HEIGHT - 1);
        let side = rng.gen_range(0..4);
        let (x, y, delta_x, delta_y) = match side {
            LEFT => (0, p, -1, 0),
            UP => (p, ARENA_HEIGHT - 1, 0, 1),
            RIGHT => (ARENA_WIDTH - 1, p, 1, 0),
            DOWN => (p, 0, 0, -1),
            _ => panic!("Bad spawn side"),
        };
        if !arena.isset(x, y) {
            break (x, y, delta_x, delta_y);
        }
    };
    spawn_snake(
        commands,
        arena,
        asset_server,
        texture_atlas_layouts,
        x,
        y,
        n,
        delta_x,
        delta_y,
    );
}

fn spawn_snake(
    mut commands: Commands,
    mut arena: ResMut<Arena>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    x: i32,
    y: i32,
    n: i32,
    delta_x: i32,
    delta_y: i32,
) {
    let (mut x, mut y) = (x, y);
    let texture = asset_server.load("snake.png");
    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::splat(40.0),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    ));
    let head_position = Position { x, y };
    let mut segments: Vec<Entity> = Vec::new();
    let segment = commands
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
        .id();
    arena.set(x, y, Occupancy::Snake(segment));
    segments.push(segment);
    for _ in 1..=n {
        x += delta_x;
        y += delta_y;
        let segment = commands
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
            .id();
        arena.set(x, y, Occupancy::Snake(segment));
        segments.push(segment);
    }
    x += delta_x;
    y += delta_y;
    let segment = commands
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
        .id();
    arena.set(x, y, Occupancy::Snake(segment));
    segments.push(segment);

    println!("Spawned segments {:?}", segments);

    let snake = commands
        .spawn((
            AI {
                move_timer: Timer::from_seconds(SNAKE_MOVEMENT_PERIOD, TimerMode::Once),
                plan_timer: Timer::from_seconds(SNAKE_PLANNING_PERIOD, TimerMode::Once),
                ..default()
            },
            Segmented {
                head_position,
                segments,
            },
            Snake,
        ))
        .id();
    println!("Snake {:?} spawned with segments", snake);
}

fn plan_rats(
    berries: Query<(Entity, &Position), With<Berry>>,
    mut rats: Query<(Entity, &mut AI, &Position), With<Rat>>,
    mut arena: ResMut<Arena>,
    time: Res<Time>,
) {
    for (rat, mut ai, position) in &mut rats {
        if !ai.plan_timer.tick(time.delta()).finished() {
            continue;
        }
        ai.plan_timer.reset();

        if ai.path.len() > 0 {
            // Already moving toward something
            continue;
        }

        let mut rng = thread_rng();
        let roll = rng.gen_range(0..10);
        ai.target = if roll <= RAT_BERRY_PREFERENCE {
            println!("Rat {:?} looking for a berry target", rat);
            choose_random_entity(&berries, &position)
        } else if roll < RAT_WANDER_PREFERENCE + RAT_BERRY_PREFERENCE {
            // Choose a random location as the target
            println!("Rat {:?} looking for a random location", rat);
            choose_random_unocc(&position, &arena)
        } else {
            None
        };

        println!(
            "Rat {:?}, position={:?}, target={:?}",
            rat, position, ai.target
        );

        if let Some(goal) = match ai.target {
            Some(Target::Entity(entity)) => Some(*berries.get(entity).unwrap().1),
            Some(Target::Position(position)) => Some(position),
            None => None,
        } {
            ai.plan_path(&position, &goal, &mut arena);
            println!("Rat {:?}, path {:?}", rat, ai.path);
        } else {
            // Target disappeared
            ai.clear();
        }
    }
}

fn plan_snakes(
    berries: Query<(Entity, &Position), With<Berry>>,
    rats: Query<(Entity, &Position), With<Rat>>,
    mut snakes: Query<(Entity, &mut AI, &Segmented), With<Snake>>,
    mut arena: ResMut<Arena>,
    time: Res<Time>,
) {
    for (snake, mut ai, segments) in &mut snakes {
        if !ai.plan_timer.tick(time.delta()).finished() {
            continue;
        }
        ai.plan_timer.reset();

        if ai.path.len() > 0 {
            continue;
        }

        let mut rng = thread_rng();
        let roll = rng.gen_range(0..10);
        ai.target = if roll <= SNAKE_RAT_PREFERENCE {
            println!("Snake {:?} looking for a rat target", snake);
            choose_random_entity(&rats, &segments.head_position)
        } else if roll <= SNAKE_BERRY_PREFERENCE + SNAKE_RAT_PREFERENCE {
            println!("Snake {:?} looking for a berry target", snake);
            choose_random_entity(&berries, &segments.head_position)
        } else if roll < SNAKE_WANDER_PREFERENCE + SNAKE_BERRY_PREFERENCE + SNAKE_RAT_PREFERENCE {
            // Choose a random location as the target
            println!("Snake {:?} looking for a random location", snake);
            choose_random_unocc(&segments.head_position, &arena)
        } else {
            None
        };

        println!(
            "Snake {:?}, head position={:?}, target={:?}",
            snake, segments.head_position, ai.target
        );
        if let Some(Target::Position(goal)) = ai.target {
            ai.plan_path(&segments.head_position, &goal, &mut arena);
            println!("Snake {:?}, path {:?}", snake, ai.path);
        } else {
            // Target disappeared
            ai.clear();
        }
    }
}

fn choose_random_entity<T: Component>(
    query: &Query<(Entity, &Position), With<T>>,
    position: &Position,
) -> Option<Target> {
    // Try to choose a random berry that's not too far away
    let mut rng = thread_rng();
    if let Some((entity, _)) = query
        .iter()
        .filter(|(_, p)| (position.x + position.y - p.x - p.y).abs() as usize <= MAX_PATH_LENGTH)
        .choose(&mut rng)
    {
        Some(Target::Entity(entity))
    } else {
        None
    }
}

fn choose_random_unocc(position: &Position, arena: &ResMut<Arena>) -> Option<Target> {
    // Limit the distance to reflect MAX_PATH_LENGTH
    let x_min = max(0, position.x - (MAX_PATH_LENGTH as i32) / 2);
    let y_min = max(0, position.y - (MAX_PATH_LENGTH as i32) / 2);
    let x_max = min(ARENA_WIDTH - 1, position.x + (MAX_PATH_LENGTH as i32) / 2);
    let y_max = min(ARENA_HEIGHT - 1, position.y + (MAX_PATH_LENGTH as i32) / 2);
    let mut rng = thread_rng();
    let mut attempts = 10;
    let (x, y) = loop {
        let (x, y) = (rng.gen_range(x_min..=x_max), rng.gen_range(y_min..=y_max));
        if !arena.isset(x, y) {
            break (x, y);
        }
        attempts += 1;
        if attempts >= 10 {
            return None;
        }
    };
    Some(Target::Position(Position { x, y }))
}

fn move_mongoose(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut mongoose: Query<(Entity, &mut Segmented), With<Mongoose>>,
    positions: Query<&mut Position, With<Mongoose>>,
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

    let (mongoose, segments) = mongoose.get_single_mut().expect("Mongoose entity missing");

    if segments.head_position.x == 0 && next_direction == LEFT {
        return;
    }
    if segments.head_position.y == ARENA_HEIGHT - 1 && next_direction == UP {
        return;
    }
    if segments.head_position.x == ARENA_WIDTH - 1 && next_direction == RIGHT {
        return;
    }
    if segments.head_position.y == 0 && next_direction == DOWN {
        return;
    }
    let (x, y) = (
        segments.head_position.x + delta_x,
        segments.head_position.y + delta_y,
    );
    match arena.occ(x, y) {
        None => move_mongoose_segments(arena, mongoose, segments, positions, delta_x, delta_y),
        Some(Occupancy::Berry(berry)) => {
            arena.unset(x, y);
            move_mongoose_segments(arena, mongoose, segments, positions, delta_x, delta_y);
            commands.entity(berry).despawn();
            scoreboard.berries_eaten_by_mongoose += 1;
            println!("Berry {:?} eaten by mongoose", berry)
        }
        Some(Occupancy::Rat(rat)) => {
            arena.unset(x, y);
            move_mongoose_segments(arena, mongoose, segments, positions, delta_x, delta_y);
            commands.entity(rat).despawn();
            scoreboard.rats_eaten_by_mongoose += 1;
            println!("Rat {:?} eaten by mongoose", rat)
        }
        Some(Occupancy::Snake(_snake)) => (), // TODO: Mongoose attacking Snakes is unimplemented
        Some(Occupancy::Mongoose(_)) => (),
    }
    input_timer.0.reset();
}

fn move_mongoose_segments(
    mut arena: ResMut<Arena>,
    entity: Entity,
    mut segmented: Mut<Segmented>,
    mut positions: Query<&mut Position, With<Mongoose>>,
    delta_x: i32,
    delta_y: i32,
) {
    segmented.head_position.x += delta_x;
    segmented.head_position.y += delta_y;
    arena.set(
        segmented.head_position.x,
        segmented.head_position.y,
        Occupancy::Mongoose(entity),
    );
    let mut gap_position = segmented.head_position.clone();
    for s in segmented.segments.iter() {
        let mut position = positions.get_mut(*s).unwrap();
        (position.x, gap_position.x) = (gap_position.x, position.x);
        (position.y, gap_position.y) = (gap_position.y, position.y);
    }
    arena.unset(gap_position.x, gap_position.y);
}

fn move_rats(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut rats: Query<(Entity, &mut AI, &mut Position), With<Rat>>,
    mut arena: ResMut<Arena>,
    time: Res<Time>,
) {
    for (rat, mut ai, mut position) in &mut rats {
        if !ai.move_timer.tick(time.delta()).finished() {
            continue;
        }
        if let Some(next_position) = ai.path.pop_front() {
            match arena.occ(next_position.x, next_position.y) {
                None => {
                    arena.unset(position.x, position.y);
                    (position.x, position.y) = (next_position.x, next_position.y);
                    arena.set(position.x, position.y, Occupancy::Rat(rat));
                }
                Some(Occupancy::Berry(berry)) => {
                    arena.unset(position.x, position.y);
                    arena.unset(next_position.x, next_position.y);
                    (position.x, position.y) = (next_position.x, next_position.y);
                    arena.set(position.x, position.y, Occupancy::Rat(rat));
                    commands.entity(berry).despawn();
                    scoreboard.berries_eaten_by_rats += 1;
                    println!("Berry {:?} eaten by rat", berry)
                }
                Some(_) => {
                    println!(
                        "Rat {:?}, position ({}, {}) is blocked",
                        rat, next_position.x, next_position.y
                    );
                    ai.clear();
                }
            }
            ai.move_timer.reset();
        }
    }
}

fn move_snakes(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut snakes: Query<(Entity, &mut AI, &mut Segmented), With<Snake>>,
    mut positions: Query<&mut Position, With<Snake>>,
    mut arena: ResMut<Arena>,
    time: Res<Time>,
) {
    for (snake, mut ai, segments) in &mut snakes {
        if !ai.move_timer.tick(time.delta()).finished() {
            continue;
        }
        if let Some(next_position) = ai.path.pop_front() {
            let (x, y) = (next_position.x, next_position.y);
            match arena.occ(x, y) {
                None => {
                    move_snake_segments(snake, next_position, segments, &mut arena, &mut positions)
                }
                Some(Occupancy::Berry(berry)) => {
                    arena.unset(x, y);
                    move_snake_segments(snake, next_position, segments, &mut arena, &mut positions);
                    commands.entity(berry).despawn();
                    scoreboard.berries_eaten_by_snakes += 1;
                    println!("Berry {:?} eaten by snake", berry)
                }
                Some(Occupancy::Rat(rat)) => {
                    arena.unset(x, y);
                    move_snake_segments(snake, next_position, segments, &mut arena, &mut positions);
                    commands.entity(rat).despawn();
                    scoreboard.rats_eaten_by_snakes += 1;
                    println!("Rat {:?} eaten by snake", rat)
                }
                Some(Occupancy::Mongoose(_)) => {
                    // Snakes attacking Mongoose is unimplemented
                    println!(
                        "Snake {:?}, position ({}, {}) is blocked by mongoose",
                        snake, next_position.x, next_position.y
                    );
                    ai.clear();
                }
                Some(Occupancy::Snake(other_snake)) => {
                    println!(
                        "Snake {:?}, position ({}, {}) is blocked by snake {:?}",
                        snake, next_position.x, next_position.y, other_snake
                    );
                    ai.clear();
                }
            }
        }
        ai.move_timer.reset();
    }
}

fn move_snake_segments(
    snake: Entity,
    next_position: Position,
    mut segments: Mut<Segmented>,
    arena: &mut ResMut<Arena>,
    positions: &mut Query<&mut Position, With<Snake>>,
) {
    segments.head_position.x = next_position.x;
    segments.head_position.y = next_position.y;
    arena.set(
        segments.head_position.x,
        segments.head_position.y,
        Occupancy::Snake(snake),
    );
    let mut gap_position = segments.head_position.clone();
    let mut grown = false;
    for s in segments.segments.iter() {
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
    things: Query<(Entity, &Segmented, Has<Mongoose>)>,
    mut segments: Query<(&Position, &mut TextureAtlas)>,
) {
    'things: for (thing, segmented, is_mongoose) in &things {
        // TODO do this only after movement, maybe check for a needs_redraw flag
        let i_tail = segmented.segments.len() - 2;
        for (i, (f, b)) in segmented.segments.iter().tuple_windows().enumerate() {
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
                panic!(
                    "{} {:?}, segment pair {}, f ({}, {}), b ({}, {}); successive segments are neither adjacent nor at the same place",
                    if is_mongoose { "Mongoose" } else { "Snake" },
                    thing,
                    i,
                    pos_f.x,
                    pos_f.y,
                    pos_b.x,
                    pos_b.y
                );
            };
            if direction == None {
                ta_f.index += TAIL;
                ta_b.index = SPRITE_SHEET_COLUMNS - 1; // Should be a blank sprite
                continue 'things;
            }
            let direction = direction.unwrap();
            if i == 0 {
                // Entity f is the head segment
                ta_f.index = HEAD + direction;
            } else {
                ta_f.index = BODY
                    + match (direction, ta_f.index) {
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
                ta_b.index = TAIL + direction;
            } else {
                ta_b.index = direction;
            }
        }
    }
}

fn grow_snakes(
    mut commands: Commands,
    mut snakes: Query<(Entity, &mut Segmented), With<Snake>>,
    positions: Query<&Position>,
    mut reader: EventReader<GrowEvent>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for event in reader.read() {
        if let Ok((snake, mut segmented)) = snakes.get_mut(event.segmented) {
            let texture = asset_server.load("snake.png");
            let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
                Vec2::splat(40.0),
                SPRITE_SHEET_COLUMNS,
                SPRITE_SHEET_ROWS,
                None,
                None,
            ));
            let tail_position = positions
                .get(
                    *segmented
                        .segments
                        .last()
                        .expect(&format!("Snake {:?}. segments vector is empty", snake)),
                )
                .expect(&format!(
                    "Snake {:?}, length {:?}. tail segment position missing",
                    snake,
                    segmented.segments.len(),
                )) // FIXME: clippy will convert this so the format occured only when an error occurs
                .clone();
            let new_segment = commands
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
                .id();
            println!("Snake {:?} got new segment {:?}", snake, new_segment);
            segmented.segments.push(new_segment);
        } else {
            panic!("Error getting snake {:?}", event.segmented); // FIXME turn this into a log after snakes can despawn
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text, With<ScoreboardUI>>) {
    let mut text = query.single_mut();
    text.sections[1].value = (scoreboard.berries_eaten_by_mongoose
        + scoreboard.rats_eaten_by_mongoose
        + scoreboard.snakes_killed)
        .to_string();
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mongoose!".into(),
                    resolution: WindowResolution::new(
                        (40 * ARENA_WIDTH) as f32,
                        (40 * ARENA_HEIGHT) as f32,
                    )
                    .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_event::<GrowEvent>()
        .insert_resource(Arena::new())
        .insert_resource(Scoreboard { ..default() })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(InputTimer(Timer::from_seconds(
            INPUT_PERIOD,
            TimerMode::Once,
        )))
        .insert_resource(BerrySpawnTimer(Timer::from_seconds(
            BERRY_SPAWN_PERIOD,
            TimerMode::Repeating,
        )))
        .insert_resource(RatSpawnTimer(Timer::from_seconds(
            RAT_SPAWN_PERIOD,
            TimerMode::Repeating,
        )))
        .insert_resource(SnakeSpawnTimer(Timer::from_seconds(
            SNAKE_SPAWN_PERIOD,
            TimerMode::Repeating,
        )))
        .add_systems(
            Startup,
            (
                spawn_camera,
                spawn_scoreboard,
                spawn_mongoose,
                //test_spawn_snake,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                spawn_rats,
                spawn_snakes,
                plan_rats,
                move_rats,
                plan_snakes,
                move_snakes,
                move_mongoose,
                grow_snakes,
                set_segment_sprites,
                spawn_berries,
                transformation,
                detect_removals,
            )
                .chain(),
        )
        .add_systems(Update, (update_scoreboard, bevy::window::close_on_esc))
        .run();
}

#[allow(dead_code)] // FIXME
fn test_spawn_snake(
    commands: Commands,
    arena: ResMut<Arena>,
    asset_server: Res<AssetServer>,
    texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let (x, y) = (3, 0);
    let n = 1;
    let (delta_x, delta_y) = (-1, 0);
    spawn_snake(
        commands,
        arena,
        asset_server,
        texture_atlas_layouts,
        x,
        y,
        n,
        delta_x,
        delta_y,
    );
}

#[allow(dead_code)] // FIXME
fn pretty_print(a: &Array2D<bool>) {
    println!();
    for y in 0..ARENA_HEIGHT as usize {
        for x in 0..ARENA_WIDTH as usize {
            print!(
                "{} ",
                if a[(x, (ARENA_HEIGHT as usize) - 1 - y)] {
                    "1"
                } else {
                    "0"
                }
            );
        }
        println!();
    }
}

fn detect_removals(mut removals: RemovedComponents<Position>) {
    for entity in removals.read() {
        // do something with the entity
        eprintln!("Entity {:?} position removed.", entity);
    }
}
