use vte::{Parser, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum AnsiNode {
	Text(String),
	Csi {
		params: Vec<u16>,
		intermediates: Vec<u8>,
		code: char,
	},
	Esc {
		intermediates: Vec<u8>,
		code: u8,
	},
	ControlChar(u8),
	Osc {
		params: Vec<Vec<u8>>,
		bell_terminated: bool,
	},
}

pub struct AstBuilder {
	pub nodes: Vec<AnsiNode>,
	current_text: String,
}

impl AstBuilder {
	fn flush_text(&mut self) {
		if !self.current_text.is_empty() {
			self.nodes.push(AnsiNode::Text(self.current_text.drain(..).collect()));
		}
	}

	pub fn parse(input: &str) -> Self {
		let mut builder = Self {
			nodes: Vec::new(),
			current_text: String::new(),
		};
		let mut parser = Parser::new();

		parser.advance(&mut builder, input.as_bytes());
		builder.flush_text();

		builder
	}
}

impl Perform for AstBuilder {
	fn print(&mut self, c: char) {
		self.current_text.push(c);
	}

	fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, code: char) {
		self.flush_text();
		let params = params.iter().flat_map(|subparams| subparams.iter().copied()).collect::<Vec<u16>>();
		self.nodes.push(AnsiNode::Csi {
			params,
			intermediates: intermediates.to_vec(),
			code,
		});
	}

	fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
		self.flush_text();
		self.nodes.push(AnsiNode::Esc {
			intermediates: intermediates.to_vec(),
			code: byte,
		});
	}

	fn execute(&mut self, byte: u8) {
		match byte {
			b'\n' => self.current_text.push('\n'),
			b'\r' => self.current_text.push('\r'),
			b'\t' => self.current_text.push('\t'),
			_ => {
				self.flush_text();
				self.nodes.push(AnsiNode::ControlChar(byte));
			},
		}
	}

	fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
		self.flush_text();
		let params = params.iter().map(|param| param.to_vec()).collect::<Vec<Vec<u8>>>();
		self.nodes.push(AnsiNode::Osc {
			params,
			bell_terminated,
		});
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn parse_single_test() {
		// 8 colors
		assert_eq!(
			AstBuilder::parse("test\x1B[33m").nodes,
			vec![
				AnsiNode::Text(String::from("test")),
				AnsiNode::Csi {
					params: vec![33],
					intermediates: Vec::new(),
					code: 'm'
				},
			]
		);

		// 16 colors
		assert_eq!(
			AstBuilder::parse("te\x1B[33;1mst").nodes,
			vec![
				AnsiNode::Text(String::from("te")),
				AnsiNode::Csi {
					params: vec![33, 1],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from("st")),
			]
		);

		// 256 colors
		assert_eq!(
			AstBuilder::parse("test \x1B[38;5;5m test").nodes,
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![38, 5, 5],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			AstBuilder::parse("test \x1B[38;5;12m test").nodes,
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![38, 5, 12],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			AstBuilder::parse("test \x1B[38;5;150m test").nodes,
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![38, 5, 150],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			AstBuilder::parse("test \x1B[38;5;241m test").nodes,
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![38, 5, 241],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from(" test")),
			]
		);

		// 24bit truecolor
		assert_eq!(
			AstBuilder::parse("\x1B[38;2;255;50;0mtest").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![38, 2, 255, 50, 0],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from("test")),
			]
		);
		assert_eq!(
			AstBuilder::parse("\x1B[38:2:255:50:0mtest").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![38, 2, 255, 50, 0],
					intermediates: Vec::new(),
					code: 'm'
				},
				AnsiNode::Text(String::from("test")),
			]
		);
	}

	#[test]
	fn test_control_chars() {
		assert_eq!(
			AstBuilder::parse("Hello\nWorld\x07").nodes,
			vec![
				AnsiNode::Text(String::from("Hello\nWorld")),
				AnsiNode::ControlChar(0x07),
			]
		);
	}

	#[test]
	fn test_osc_sequence() {
		assert_eq!(
			AstBuilder::parse("\x1B]0;Terminal Title\x07").nodes,
			vec![AnsiNode::Osc {
				params: vec![vec![b'0'], b"Terminal Title".to_vec()],
				bell_terminated: true,
			}]
		);

		assert_eq!(
			AstBuilder::parse("\x1B]0;Terminal Title\x1B\\").nodes,
			vec![
				AnsiNode::Osc {
					params: vec![vec![b'0'], b"Terminal Title".to_vec()],
					bell_terminated: false,
				},
				AnsiNode::Esc {
					intermediates: Vec::new(),
					code: 92
				}
			]
		);

		assert_eq!(
			AstBuilder::parse("\x1B]4;1;rgb:ff/00/00\x07").nodes,
			vec![AnsiNode::Osc {
				params: vec![vec![b'4'], vec![b'1'], b"rgb:ff/00/00".to_vec()],
				bell_terminated: true,
			}]
		);

		assert_eq!(
			AstBuilder::parse("\x1B]8;id=xyz;http://example.com\x07").nodes,
			vec![AnsiNode::Osc {
				params: vec![vec![b'8'], b"id=xyz".to_vec(), b"http://example.com".to_vec()],
				bell_terminated: true,
			}]
		);
	}

	#[test]
	fn test_edge_cases() {
		// Empty parameters - terminals often treat as reset/default
		assert_eq!(
			AstBuilder::parse("\x1B[m").nodes,
			vec![AnsiNode::Csi {
				params: vec![0],
				intermediates: vec![],
				code: 'm'
			}]
		);

		// Empty parameter in the middle
		assert_eq!(
			AstBuilder::parse("\x1B[1;;3m").nodes,
			vec![AnsiNode::Csi {
				params: vec![1, 0, 3],
				intermediates: vec![],
				code: 'm'
			}]
		);

		// Incomplete sequence at end
		assert_eq!(AstBuilder::parse("text\x1B[38").nodes, vec![AnsiNode::Text(String::from("text"))]);

		// Multiple sequences without text between
		assert_eq!(
			AstBuilder::parse("\x1B[1m\x1B[2m\x1B[3m").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![1],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Csi {
					params: vec![2],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Csi {
					params: vec![3],
					intermediates: vec![],
					code: 'm'
				},
			]
		);

		// Very large parameter values
		assert_eq!(
			AstBuilder::parse("\x1B[9999;65535m").nodes,
			vec![AnsiNode::Csi {
				params: vec![9999, 65535],
				intermediates: vec![],
				code: 'm'
			}]
		);

		// Mixed text and escapes with no spacing
		assert_eq!(
			AstBuilder::parse("a\x1B[31mb\x1B[0mc").nodes,
			vec![
				AnsiNode::Text(String::from("a")),
				AnsiNode::Csi {
					params: vec![31],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("b")),
				AnsiNode::Csi {
					params: vec![0],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("c")),
			]
		);

		// CSI with intermediate bytes (like CSI ? sequences)
		assert_eq!(
			AstBuilder::parse("\x1B[?25h").nodes,
			vec![AnsiNode::Csi {
				params: vec![25],
				intermediates: vec![b'?'],
				code: 'h'
			}]
		);

		// Control characters mixed with escapes
		assert_eq!(
			AstBuilder::parse("\x07\x1B[31m\x08").nodes,
			vec![
				AnsiNode::ControlChar(0x07),
				AnsiNode::Csi {
					params: vec![31],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::ControlChar(0x08),
			]
		);

		// Malformed escape that looks like ESC but isn't a sequence
		assert_eq!(
			AstBuilder::parse("\x1BZ").nodes,
			vec![AnsiNode::Esc {
				intermediates: vec![],
				code: b'Z'
			}]
		);
	}

	#[test]
	fn real_world_sequences_test() {
		// Git diff colors
		assert_eq!(
			AstBuilder::parse("\x1B[1;32m+added line\x1B[m").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![1, 32],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("+added line")),
				AnsiNode::Csi {
					params: vec![0],
					intermediates: vec![],
					code: 'm'
				},
			]
		);

		// Prompt with multiple styles
		assert_eq!(
			AstBuilder::parse("\x1B[1;34muser\x1B[0m@\x1B[1;32mhost\x1B[0m:").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![1, 34],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("user")),
				AnsiNode::Csi {
					params: vec![0],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("@")),
				AnsiNode::Csi {
					params: vec![1, 32],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("host")),
				AnsiNode::Csi {
					params: vec![0],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from(":")),
			]
		);

		// 256 color with reset
		assert_eq!(
			AstBuilder::parse("\x1B[38;5;196mRED\x1B[0m").nodes,
			vec![
				AnsiNode::Csi {
					params: vec![38, 5, 196],
					intermediates: vec![],
					code: 'm'
				},
				AnsiNode::Text(String::from("RED")),
				AnsiNode::Csi {
					params: vec![0],
					intermediates: vec![],
					code: 'm'
				},
			]
		);
	}
}
