use bevy::{
	core::FixedTimestep,
	app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
	ecs::schedule::ReportExecutionOrderAmbiguities,
	input::{keyboard::KeyCode, Input},
	log::LogPlugin,
	prelude::*,
	utils::Duration,
};
use rand::random;

const ARENA_WIDTH: u32 = 100;
const ARENA_HEIGHT: u32 = 100;

struct Player {
	name: String,
	head: PlayerHead,
}
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct Position {
    x: i32,
    y: i32,
}

struct PlayerSegment;

struct GrowthEvent;

struct GameOverEvent;

#[derive(Default)]
struct PlayerSegments(Vec<Entity>);

#[derive(Default)]
struct LastTailPosition(Option<Position>);

struct BoxSize {
    width: f32,
    height: f32,
}
impl BoxSize {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PlayerMovement {
    Input,
    Movement,
    Growth,
	Spawn,
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

struct Score {
	value: usize,
}

// RESOURCES: "Global" state accessible by systems. These are also just normal Rust data types.
#[derive(Default)]
struct GameState {
	current_round: usize,
	total_players: usize,
	winning_player: Option<String>,
}

#[derive(SystemLabel, Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
	MainMenu,
	InGame,
	Paused,
	GameOver
}

struct PlayerHead {
	direction: Direction,
}
struct Materials {
	head_material: Handle<ColorMaterial>,
	segment_material: Handle<ColorMaterial>,
}

struct GameRules {
	winning_score: usize,
	max_rounds: usize,
	max_players: usize,
}
struct MenuData {
    button_entity: Entity,
}

// SYSTEMS: Logic that runs on entities, components, and resources. These generally run once each
// time the app updates.

fn new_round_system(game_rules: Res<GameRules>, mut game_state: ResMut<GameState>) {
	game_state.current_round += 1;
	println!(
		"Begin round {} of {}",
		game_state.current_round, game_rules.max_rounds
	);
}

// Menu
fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    let button_entity = commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                // center button
                margin: Rect::all(Val::Auto),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            //material: button_materials.normal.clone(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    "Play",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                    Default::default(),
                ),
                ..Default::default()
            });
        })
        .id();
    commands.insert_resource(MenuData { button_entity });
}

fn menu(
    mut state: ResMut<State<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut material) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                state.set(AppState::InGame).unwrap();
            }
            Interaction::Hovered => {
				println!("{:?}", state.current());
            }
            Interaction::None => {
                println!("hovered");
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>) {
    commands.entity(menu_data.button_entity).despawn_recursive();
}

fn change_color(
    time: Res<Time>,
    mut assets: ResMut<Assets<ColorMaterial>>,
    query: Query<&Handle<ColorMaterial>, With<Sprite>>,
) {
    for handle in query.iter() {
        let material = assets.get_mut(handle).unwrap();
        material
            .color
            .set_b((time.seconds_since_startup() * 5.0).sin() as f32 + 2.0);
    }
}

// This system updates the score for each entity with the "Player" and "Score" component.
fn score_system(mut query: Query<(&Player, &mut Score)>) {
	for (player, mut score) in query.iter_mut() {
		let scored_a_point = random::<bool>();
		if scored_a_point {
			score.value += 1;
			println!(
				"{} scored a point! Their score is: {}",
				player.name, score.value
			);
		} else {
			println!(
				"{} did not score a point! Their score is: {}",
				player.name, score.value
			);
		}
	}
}

// Scaling sprites
fn size_scaling(windows: Res<Windows>, mut q: Query<(&BoxSize, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in q.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

// This system runs on all entities with the "Player" and "Score" components, but it also
// accesses the "GameRules" resource to determine if a player has won.
fn score_check_system(
	game_rules: Res<GameRules>,
	mut game_state: ResMut<GameState>,
	query: Query<(&Player, &Score)>,
) {
	for (player, score) in query.iter() {
		if score.value == game_rules.winning_score {
			game_state.winning_player = Some(player.name.clone());
		}
	}
}

// This system ends the game if we meet the right conditions. This fires an AppExit event, which
// tells our App to quit. Check out the "event.rs" example if you want to learn more about using
// events.
fn game_over_system(
	game_rules: Res<GameRules>,
	game_state: Res<GameState>,
	mut app_exit_events: EventWriter<AppExit>,
) {
	if let Some(ref player) = game_state.winning_player {
		println!("{} won the game!", player);
		app_exit_events.send(AppExit);
	} else if game_state.current_round == game_rules.max_rounds {
		println!("Ran out of rounds. Nobody wins!");
		app_exit_events.send(AppExit);
	}
}

// This is a "startup" system that runs exactly once when the app starts up. Startup systems are
// generally used to create the initial "state" of our game. The only thing that distinguishes a
// "startup" system from a "normal" system is how it is registered:      Startup:
// app.add_startup_system(startup_system)      Normal:  app.add_system(normal_system)
fn startup_system(
	mut commands: Commands,
	mut game_state: ResMut<GameState>,
	mut materials: ResMut<Assets<ColorMaterial>>,
) {
	// Create our game rules resource
	commands.insert_resource(GameRules {
		max_rounds: 100,
		winning_score: 51,
		max_players: 4,
	});
	commands.spawn_batch(vec![
		(
			Player {
				name: "Quorra".to_string(),
				head: PlayerHead {direction: Direction::Up},
			},
			Score { value: 0 },
		),
		(
			Player {
				name: "Clu".to_string(),
				head: PlayerHead {direction: Direction::Down},
			},
			Score { value: 0 },
		),
	]);
	// Create a camera
	commands.spawn_bundle(OrthographicCameraBundle::new_2d());
	commands.insert_resource(Materials {
        head_material: materials.add(Color::rgb(0.1, 0.9, 0.9).into()),
		segment_material: materials.add(Color::rgb(0.1, 0.7, 0.7).into())
    });
	game_state.total_players = 2;
}

// This system uses a command buffer to (potentially) add a new player to our game on each
// iteration. Normal systems cannot safely access the World instance directly because they run in
// parallel. Our World contains all of our components, so mutating arbitrary parts of it in parallel
// is not thread safe. Command buffers give us the ability to queue up changes to our World without
// directly accessing it
fn new_player_system(
	mut commands: Commands,
	game_rules: Res<GameRules>,
	mut game_state: ResMut<GameState>,
) {
	let add_new_player = random::<bool>();
	if add_new_player && game_state.total_players < game_rules.max_players {
		game_state.total_players += 1;
		commands.spawn_bundle((
			Player {
				name: format!("Player {}", game_state.total_players),
				head: PlayerHead {direction: Direction::Down},
			},
			Score { value: 0 },
		));

		println!("Player {} joined the game!", game_state.total_players);
	}
}

// Spawn new tron player
fn spawn_player(
    mut commands: Commands,
    materials: Res<Materials>,
    mut segments: ResMut<PlayerSegments>,
) {
    segments.0 = vec![
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.head_material.clone(),
                sprite: Sprite::new(Vec2::new(10.0, 10.0)),
                ..Default::default()
            })
            .insert(PlayerHead {
                direction: Direction::Up,
            })
            .insert(PlayerSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(BoxSize::square(0.8))
            .id(),
        spawn_segment(
            commands,
            &materials.segment_material,
            Position { x: 3, y: 2 },
        ),
    ];
}

// Move player
fn player_movement_input(keyboard_input: Res<Input<KeyCode>>, mut heads: Query<&mut PlayerHead>, state: ResMut<State<AppState>>,) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::Left) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::Down) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::Up) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::Right) {
            Direction::Right
        } else {
            head.direction
        };
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
	println!("{:?}", state.current());
}

fn player_movement(
    segments: ResMut<PlayerSegments>,
    mut heads: Query<(Entity, &PlayerHead)>,
    mut positions: Query<&mut Position>,
	mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if let Some((head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .0
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();
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
		if segment_positions.contains(&head_pos) {
			game_over_writer.send(GameOverEvent);
		}
		if head_pos.x < 0
			|| head_pos.y < 0
			|| head_pos.x as u32 >= ARENA_WIDTH
			|| head_pos.y as u32 >= ARENA_HEIGHT
		{
			game_over_writer.send(GameOverEvent);
		}
        segment_positions
            .iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });
    }
}

fn player_growth(
    commands: Commands,
    head_positions: Query<&Position, With<PlayerHead>>,
    mut segments: ResMut<PlayerSegments>,
    materials: Res<Materials>,
) {
	//println!("\n{:?}\n", head_positions);
	segments.0.push(spawn_segment( // This would add the tail always to the same player
		commands,
		&materials.segment_material,
		head_positions.single().unwrap().clone().into(),
	));
}

fn spawn_segment(
    mut commands: Commands,
    material: &Handle<ColorMaterial>,
    position: Position,
) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            material: material.clone(),
            ..Default::default()
        })
        .insert(PlayerSegment)
        .insert(position)
        .insert(BoxSize::square(0.65))
        .id()
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    materials: Res<Materials>,
	players: Query<Entity, With<Position>>,
    segments_res: ResMut<PlayerSegments>,
    segments: Query<Entity, With<PlayerSegment>>,
) {
    if reader.iter().next().is_some() {
		for ent in players.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_player(commands, materials, segments_res); // Before this line delete the player trail
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum MyStage {
	BeforeRound,
	AfterRound,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum MyLabels {
	ScoreCheck,
}

// Our Bevy app's entry point
fn main() {
	// Bevy apps are created using the builder pattern. We use the builder to add systems,
	// resources, and plugins to our app
	App::build()
		// Resize and rename window
		.insert_resource(WindowDescriptor { // <--
			title: "Nuisance Value".to_string(), // <--
			width: 500.0,                 // <--
			height: 500.0,                // <--
			..Default::default()         // <--
		})
		.add_state(AppState::MainMenu)
		// Change colors
		.insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
		// Player tails
		.insert_resource(PlayerSegments::default())
		.insert_resource(LastTailPosition::default())
		// Some systems are configured by adding their settings as a resource
		.insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs(5)))
		// Plugins are just a grouped set of app builder calls (just like we're doing here).
		// We could easily turn our game into a plugin, but you can check out the plugin example for
		// that :) The plugin below runs our app's "system schedule" once every 5 seconds
		// (configured above).
		.add_plugin(ScheduleRunnerPlugin::default())
		// Resources that implement the Default or FromResources trait can be added like this:
		.init_resource::<GameState>()
		// Startup systems run exactly once BEFORE all other systems. These are generally used for
		// app initialization code (ex: adding entities and resources)
		//.add_startup_system(startup_system.system())
		// Add Player death
		.add_event::<GameOverEvent>()
		// Add tail event
		.add_event::<GrowthEvent>()
		// Add game setup to stage
		//.add_startup_stage("game_setup", SystemStage::single(spawn_player.system()))
		// SYSTEM EXECUTION ORDER
		//
		// Each system belongs to a `Stage`, which controls the execution strategy and broad order
		// of the systems within each tick. Startup stages (which startup systems are
		// registered in) will always complete before ordinary stages begin,
		// and every system in a stage must complete before the next stage advances.
		// Once every stage has concluded, the main loop is complete and begins again.
		//
		// By default, all systems run in parallel, except when they require mutable access to a
		// piece of data. This is efficient, but sometimes order matters.
		// For example, we want our "game over" system to execute after all other systems to ensure
		// we don't accidentally run the game for an extra round.
		//
		// Rather than splitting each of your systems into separate stages, you should force an
		// explicit ordering between them by giving the relevant systems a label with
		// `.label`, then using the `.before` or `.after` methods. Systems will not be
		// scheduled until all of the systems that they have an "ordering dependency" on have
		// completed.
		//
		// Doing that will, in just about all cases, lead to better performance compared to
		// splitting systems between stages, because it gives the scheduling algorithm more
		// opportunities to run systems in parallel.
		// Stages are still necessary, however: end of a stage is a hard sync point
		// (meaning, no systems are running) where `Commands` issued by systems are processed.
		// This is required because commands can perform operations that are incompatible with
		// having systems in flight, such as spawning or deleting entities,
		// adding or removing resources, etc.
		//
		// add_system(system) adds systems to the UPDATE stage by default
		// However we can manually specify the stage if we want to. The following is equivalent to
		// add_system(score_system)

		//.add_system_to_stage(CoreStage::Update, score_system.system())

		// We can also create new stages. Here is what our games stage order will look like:
		// "before_round": new_player_system, new_round_system
		// "update": print_message_system, score_system
		// "after_round": score_check_system, game_over_system
		.add_stage_before(
			CoreStage::Update,
			MyStage::BeforeRound,
			SystemStage::parallel(),
		)
		.add_stage_after(
			CoreStage::Update,
			MyStage::AfterRound,
			SystemStage::parallel(),
		)
		
		//.add_system_to_stage(MyStage::BeforeRound, new_round_system.system())
		//.add_system_to_stage(MyStage::BeforeRound, new_player_system.system())

		// We can ensure that game_over system runs after score_check_system using explicit ordering
		// constraints First, we label the system we want to refer to using `.label`
		// Then, we use either `.before` or `.after` to describe the order we want the relationship
		/*
		.add_system_to_stage(
			MyStage::AfterRound,
			score_check_system.system().label(MyLabels::ScoreCheck),
		)*/
		/*
		.add_system_to_stage(
			MyStage::AfterRound,
			game_over_system.system().after(MyLabels::ScoreCheck),
		)*/
		//.add_system(game_over.system().before(PlayerMovement::Movement))
		.add_system_set_to_stage(
			CoreStage::PreUpdate,
			SystemSet::on_enter(AppState::MainMenu)
				.with_system(startup_system.system().label(AppState::MainMenu))
				.with_system(setup_menu.system().label(AppState::MainMenu))
		)
        .add_system_set_to_stage(
			CoreStage::PreUpdate,
			SystemSet::on_update(AppState::MainMenu)
				.with_system(menu.system().label(AppState::MainMenu))
		)
        .add_system_set_to_stage(
			CoreStage::PreUpdate,
			SystemSet::on_exit(AppState::MainMenu)
				.with_system(cleanup_menu.system().label(AppState::MainMenu))
		)
        .add_system_set_to_stage(
			CoreStage::Update,
			SystemSet::on_enter(AppState::InGame)
				.with_system(position_translation.system())
				.with_system(size_scaling.system())
				.with_system(
					spawn_player
					.system()
					.label(PlayerMovement::Spawn)
					.before(PlayerMovement::Movement)
				)
		)
        .add_system_set_to_stage(
			CoreStage::PostUpdate,
            SystemSet::on_update(AppState::InGame)
				.with_run_criteria(FixedTimestep::step(0.150))
				.with_system(
					player_growth
					.system()
					.label(PlayerMovement::Growth)
					.after(PlayerMovement::Movement),
				)
				.with_system(
					player_movement_input
					.system()
					.label(PlayerMovement::Input)
					.before(PlayerMovement::Movement),
				)
				.with_system(player_movement.system().label(PlayerMovement::Movement))

        )
		.insert_resource(ReportExecutionOrderAmbiguities)
		.add_plugins(DefaultPlugins)
		// This call to run() starts the app we just built!
		.run();
}
