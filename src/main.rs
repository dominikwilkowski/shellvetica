// use std::io::{Read, stdin};

// mod shellvetica;

// use crate::shellvetica::Shellvetica;

use vte::{Parser, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum AnsiNode {
	Text(String),
	Csi { params: Vec<u16>, code: char },
	Esc { intermediates: Vec<u8>, code: char },
}

struct AstBuilder {
	nodes: Vec<AnsiNode>,
	current_text: String,
}

impl AstBuilder {
	fn new() -> Self {
		Self {
			nodes: Vec::new(),
			current_text: String::new(),
		}
	}

	fn flush_text(&mut self) {
		if !self.current_text.is_empty() {
			self.nodes.push(AnsiNode::Text(self.current_text.drain(..).collect()));
		}
	}
}

impl Perform for AstBuilder {
	fn print(&mut self, c: char) {
		self.current_text.push(c);
	}

	fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, code: char) {
		self.flush_text();

		let params: Vec<u16> = params.iter().map(|p| p[0]).collect();

		self.nodes.push(AnsiNode::Csi { params, code });
	}

	fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
		self.flush_text();

		self.nodes.push(AnsiNode::Esc {
			intermediates: intermediates.to_vec(),
			code: byte as char,
		});
	}
}

fn parse_ansi(input: &str) -> Vec<AnsiNode> {
	let mut builder = AstBuilder::new();
	let mut parser = Parser::new();

	parser.advance(&mut builder, input.as_bytes());
	builder.flush_text();

	builder.nodes
}

fn main() {
	// let mut buffer = Vec::new();
	// match stdin().read_to_end(&mut buffer) {
	// 	Ok(_) => {},
	// 	Err(error) => panic!("Failed to read buffer: {error:?}"),
	// }
	// let input = String::from_utf8_lossy(&buffer);

	// let html = Shellvetica::convert(&input).export();
	// println!("{html}");

	let input = "\x1B[38;2;255;50;0mtest\x1B[0m";
	println!("{input} => {:?}", parse_ansi(input));

	let input = "\x1B[33mtest\x1B[39m";
	println!("{input} => {:?}", parse_ansi(input));

	let input = "\x1B[35mtest";
	println!("{input}\x1B[39m => {:?}", parse_ansi(input));
}
