use rand::prelude::random;

use bevy::{
    math::bounding::{Aabb2d, BoundingCircle, IntersectsVolume},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const MONGOOSE_SIZE: Vec2 = Vec2::new(20.0, 20.0);
const MONGOOSE_SPEED: f32 = 200.0;

const BERRY_DIAMETER: f32 = 15.0;

const BACKGROUND_COLOR: Color = Color::rgb(0.6, 0.9, 0.2);
const MONGOOSE_COLOR: Color = Color::rgb(0.8, 0.6, 0.0);
const BERRY_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

#[derive(Component)]
struct MongooseHead;

#[derive(Component)]
struct MongooseBody;

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
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
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
        Velocity(Vec2::new(0.0, 0.0)),
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
        ]),
    ));
}

fn spawn_berry(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<BerrySpawnTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(Circle::default()).into(),
                material: materials.add(BERRY_COLOR),
                transform: Transform::from_translation(Vec3::new(
                    random::<f32>()*200.0-100.0,
                    random::<f32>()*200.0-100.0,
                    1.0
                ))
                    .with_scale(Vec2::splat(BERRY_DIAMETER).extend(1.0)),
                ..default()
            },
            Berry,
            Collider,
        ));
    }
}

fn setup(
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_mongoose(&mut commands);
    spawn_scoreboard(&mut commands);
}


fn move_mongoose(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MongooseHead>>,
    time: Res<Time>,
) {
    let mut mongoose_transform = query.single_mut();
    let mut x_direction = 0.0;
    let mut y_direction = 0.0;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        x_direction -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        x_direction += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        y_direction += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        y_direction -= 1.0;
    }

    let new_mongoose_x_position =
        mongoose_transform.translation.x + x_direction*MONGOOSE_SPEED*time.delta_seconds();
    let new_mongoose_y_position =
        mongoose_transform.translation.y + y_direction*MONGOOSE_SPEED*time.delta_seconds();

    // TODO bounding
    mongoose_transform.translation.x = new_mongoose_x_position;
    mongoose_transform.translation.y = new_mongoose_y_position;
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_seconds();
        transform.translation.y += velocity.y * time.delta_seconds();
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
        .add_plugins(DefaultPlugins)
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
                apply_velocity,
                move_mongoose,
                check_for_collisions,
                spawn_berry,
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
