use std::io::{Read, stdin};

mod shellvetica;

use crate::shellvetica::Shellvetica;

fn main() {
	let mut buffer = Vec::new();
	match stdin().read_to_end(&mut buffer) {
		Ok(_) => {},
		Err(error) => panic!("Failed to read buffer: {error:?}"),
	}
	let input = String::from_utf8_lossy(&buffer);

	let html = Shellvetica::convert(&input).export();
	println!("{html}");
}
