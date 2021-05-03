use bevy::prelude::*;

fn henlo() {
	println!("Henlo");
}
fn main() {
	App::build()
		.add_system(henlo.system())
		.run();
}
