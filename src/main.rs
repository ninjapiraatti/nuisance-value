use bevy::{
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
}
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Position {
    x: i32,
    y: i32,
}

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

struct PlayerHead;
struct Materials {
	head_material: Handle<ColorMaterial>,
}

struct GameRules {
	winning_score: usize,
	max_rounds: usize,
	max_players: usize,
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

fn keyboard_input_system(keyboard_input: Res<Input<KeyCode>>) {
	if keyboard_input.pressed(KeyCode::A) {
		info!("'A' currently pressed");
	}

	if keyboard_input.just_pressed(KeyCode::A) {
		info!("'A' just pressed");
	}

	if keyboard_input.just_released(KeyCode::A) {
		info!("'A' just released");
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
fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
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
		max_rounds: 100000,
		winning_score: 40000,
		max_players: 4,
	});
	commands.spawn_batch(vec![
		(
			Player {
				name: "Alice".to_string(),
			},
			Score { value: 0 },
		),
		(
			Player {
				name: "Bob".to_string(),
			},
			Score { value: 0 },
		),
	]);
	// Create a camera
	commands.spawn_bundle(OrthographicCameraBundle::new_2d());
	commands.insert_resource(Materials {
        head_material: materials.add(Color::rgb(0.1, 0.9, 0.9).into()),
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
			},
			Score { value: 0 },
		));

		println!("Player {} joined the game!", game_state.total_players);
	}
}

// Spawn new tron player
fn spawn_player(mut commands: Commands, materials: Res<Materials>) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.head_material.clone(),
            sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
        })
        .insert(PlayerHead)
		.insert(Position { x: 3, y: 3 })
		.insert(Size::square(0.8));
}

// Move player
fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut head_positions: Query<&mut Position, With<PlayerHead>>,
) {
    for mut pos in head_positions.iter_mut() {
        if keyboard_input.pressed(KeyCode::Left) {
            pos.x -= 2;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            pos.x += 2;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            pos.y -= 2;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            pos.y += 2;
        }
    }
}

// If you really need full, immediate read/write access to the world or resources, you can use a
// "thread local system". These run on the main app thread (hence the name "thread local")
// WARNING: These will block all parallel execution of other systems until they finish, so they
// should generally be avoided if you care about performance
#[allow(dead_code)]
fn thread_local_system(world: &mut World) {
	// this does the same thing as "new_player_system"
	let total_players = world.get_resource_mut::<GameState>().unwrap().total_players;
	let should_add_player = {
		let game_rules = world.get_resource::<GameRules>().unwrap();
		let add_new_player = random::<bool>();
		add_new_player && total_players < game_rules.max_players
	};
	// Randomly add a new player
	if should_add_player {
		world.spawn().insert_bundle((
			Player {
				name: format!("Player {}", total_players),
			},
			Score { value: 0 },
		));

		let mut game_state = world.get_resource_mut::<GameState>().unwrap();
		game_state.total_players += 1;
	}
}

// Sometimes systems need their own unique "local" state. Bevy's ECS provides Local<T> resources for
// this case. Local<T> resources are unique to their system and are automatically initialized on
// your behalf (if they don't already exist). If you have a system's id, you can also access local
// resources directly in the Resources collection using `Resources::get_local()`. In general you
// should only need this feature in the following cases:  1. You have multiple instances of the same
// system and they each need their own unique state  2. You already have a global version of a
// resource that you don't want to overwrite for your current system  3. You are too lazy to
// register the system's resource as a global resource

#[derive(Default)]
struct State {
	counter: usize,
}

// NOTE: this doesn't do anything relevant to our game, it is just here for illustrative purposes
#[allow(dead_code)]
fn local_state_system(mut state: Local<State>, query: Query<(&Player, &Score)>) {
	for (player, score) in query.iter() {
		println!("processed: {} {}", player.name, score.value);
	}
	println!("this system ran {} times", state.counter);
	state.counter += 1;
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
		// Resources can be added to our app like this
		.insert_resource(State { counter: 0 })
		// Change colors
		.insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
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
		.add_startup_system(startup_system.system())
		// Add game setup to stage
		.add_startup_stage("game_setup", SystemStage::single(spawn_player.system()))
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
		.add_system_to_stage(CoreStage::Update, score_system.system())
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
		.add_system_to_stage(MyStage::BeforeRound, new_round_system.system())
		.add_system_to_stage(MyStage::BeforeRound, new_player_system.system())
		// We can ensure that game_over system runs after score_check_system using explicit ordering
		// constraints First, we label the system we want to refer to using `.label`
		// Then, we use either `.before` or `.after` to describe the order we want the relationship
		.add_system_to_stage(
			MyStage::AfterRound,
			score_check_system.system().label(MyLabels::ScoreCheck),
		)
		.add_system_to_stage(
			MyStage::AfterRound,
			game_over_system.system().after(MyLabels::ScoreCheck),
		)
		.add_system_to_stage(
			MyStage::AfterRound,
			keyboard_input_system.system().after(MyLabels::ScoreCheck),
		)
		.add_system(player_movement.system())
		.add_system_set_to_stage(
			CoreStage::PostUpdate,
			SystemSet::new()
				.with_system(position_translation.system())
				.with_system(size_scaling.system()),
		)
		.add_plugins(DefaultPlugins)
		.insert_resource(ReportExecutionOrderAmbiguities)
		// This call to run() starts the app we just built!
		.run();
}
