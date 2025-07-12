use vte::{Parser, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum AnsiNode {
	Text(String),
	Csi {
		params: Vec<Vec<u16>>,
		intermediates: Vec<u8>,
		code: char,
	},
	Esc {
		intermediates: Vec<u8>,
		byte: u8,
	},
	ControlChar(u8),
	Osc {
		params: Vec<Vec<u8>>,
		bell_terminated: bool,
	},
}

impl AnsiNode {
	pub fn is_zero_width(&self) -> bool {
		match self {
			AnsiNode::Text(s) => s.is_empty(),
			AnsiNode::ControlChar(b) => matches!(b,
					b'\x00'..=b'\x08' | b'\x0B'..=b'\x0C' | b'\x0E'..=b'\x1F' | b'\x7F'
			),
			_ => false,
		}
	}

	pub fn is_cursor_movement(&self) -> bool {
		match self {
			AnsiNode::Csi { code, .. } => {
				matches!(code, 'H' | 'J' | 'K' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'S' | 'T' | 'f' | 's' | 'u')
			},
			_ => false,
		}
	}
}

pub struct TerminalOutputParser {
	nodes: Vec<AnsiNode>,
	current_text: String,
}

impl TerminalOutputParser {
	fn flush_text(&mut self) {
		if !self.current_text.is_empty() {
			self.nodes.push(AnsiNode::Text(std::mem::take(&mut self.current_text)));
		}
	}

	fn normalize_crlf(input: &[u8]) -> Vec<u8> {
		let mut normalized = Vec::with_capacity(input.len());
		let mut i = 0;
		while i < input.len() {
			if i + 1 < input.len() && input[i] == b'\r' && input[i + 1] == b'\n' {
				normalized.push(b'\n');
				i += 2;
			} else {
				normalized.push(input[i]);
				i += 1;
			}
		}
		normalized
	}

	pub fn parse_to_nodes(input: &[u8]) -> Vec<AnsiNode> {
		let needs_normalization = input.windows(2).any(|w| w == b"\r\n");
		let normalized_storage;
		let input_to_parse = if needs_normalization {
			normalized_storage = Self::normalize_crlf(input);
			&normalized_storage
		} else {
			input
		};

		let estimated_text_size = input_to_parse.len() * 8 / 10;
		let mut builder = Self {
			nodes: Vec::new(),
			current_text: String::with_capacity(estimated_text_size),
		};
		let mut parser = Parser::new();

		parser.advance(&mut builder, input_to_parse);
		builder.flush_text();

		builder.nodes
	}
}

impl Perform for TerminalOutputParser {
	fn print(&mut self, c: char) {
		self.current_text.push(c);
	}

	fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, code: char) {
		self.flush_text();

		let params_vec = params.iter().map(|subparams| subparams.to_vec()).collect::<Vec<Vec<u16>>>();

		self.nodes.push(AnsiNode::Csi {
			params: params_vec,
			intermediates: intermediates.to_vec(),
			code,
		});
	}

	fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
		self.flush_text();
		self.nodes.push(AnsiNode::Esc {
			intermediates: intermediates.to_vec(),
			byte,
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

		let mut params_vec = Vec::with_capacity(params.len());
		for param in params {
			params_vec.push(param.to_vec());
		}

		self.nodes.push(AnsiNode::Osc {
			params: params_vec,
			bell_terminated,
		});
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn parse_single_test() {
		// styles
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[4m test "),
			vec![
				AnsiNode::Csi {
					params: vec![vec![4]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test ")),
			]
		);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[4:2m test "),
			vec![
				AnsiNode::Csi {
					params: vec![vec![4, 2]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test ")),
			]
		);

		// 8 colors
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"test\x1B[33m"),
			vec![
				AnsiNode::Text(String::from("test")),
				AnsiNode::Csi {
					params: vec![vec![33]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);

		// 16 colors
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"te\x1B[33;1mst"),
			vec![
				AnsiNode::Text(String::from("te")),
				AnsiNode::Csi {
					params: vec![vec![33], vec![1]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("st")),
			]
		);

		// 256 colors
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"test \x1B[38;5;5m test"),
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![vec![38], vec![5], vec![5]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"test \x1B[38;5;12m test"),
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![vec![38], vec![5], vec![12]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"test \x1B[38;5;150m test"),
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![vec![38], vec![5], vec![150]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test")),
			]
		);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"test \x1B[38;5;241m test"),
			vec![
				AnsiNode::Text(String::from("test ")),
				AnsiNode::Csi {
					params: vec![vec![38], vec![5], vec![241]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(" test")),
			]
		);

		// 24bit truecolor
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[38;2;255;50;0mtest"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![38], vec![2], vec![255], vec![50], vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("test")),
			]
		);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[38:2:255:50:0mtest"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![38, 2, 255, 50, 0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("test")),
			]
		);
	}

	#[test]
	fn test_control_chars() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"Hello\nWorld\x07"),
			vec![
				AnsiNode::Text(String::from("Hello\nWorld")),
				AnsiNode::ControlChar(0x07),
			]
		);
	}

	#[test]
	fn osc_sequence_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B]0;Terminal Title\x07"),
			vec![AnsiNode::Osc {
				params: vec![vec![b'0'], b"Terminal Title".to_vec()],
				bell_terminated: true,
			}]
		);

		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B]0;Terminal Title\x1B\\"),
			vec![
				AnsiNode::Osc {
					params: vec![vec![b'0'], b"Terminal Title".to_vec()],
					bell_terminated: false,
				},
				AnsiNode::Esc {
					intermediates: vec![],
					byte: 92,
				},
			]
		);

		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B]4;1;rgb:ff/00/00\x07"),
			vec![AnsiNode::Osc {
				params: vec![vec![b'4'], vec![b'1'], b"rgb:ff/00/00".to_vec()],
				bell_terminated: true,
			}]
		);

		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B]8;id=xyz;http://example.com\x07"),
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
			TerminalOutputParser::parse_to_nodes(b"\x1B[m"),
			vec![AnsiNode::Csi {
				params: vec![vec![0]],
				intermediates: vec![],
				code: 'm',
			}]
		);

		// Empty parameter in the middle
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[1;;3m"),
			vec![AnsiNode::Csi {
				params: vec![vec![1], vec![0], vec![3]],
				intermediates: vec![],
				code: 'm',
			}]
		);

		// Incomplete sequence at end
		assert_eq!(TerminalOutputParser::parse_to_nodes(b"text\x1B[38"), vec![AnsiNode::Text(String::from("text"))]);

		// Multiple sequences without text between
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[1m\x1B[2m\x1B[3m"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![1]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![2]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![3]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);

		// Very large parameter values
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[9999;65535m"),
			vec![AnsiNode::Csi {
				params: vec![vec![9999], vec![65535]],
				intermediates: vec![],
				code: 'm',
			}]
		);

		// Mixed text and escapes with no spacing
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"a\x1B[31mb\x1B[0mc"),
			vec![
				AnsiNode::Text(String::from("a")),
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("b")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("c")),
			]
		);

		// CSI with intermediate bytes (like CSI ? sequences)
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[?25h"),
			vec![AnsiNode::Csi {
				params: vec![vec![25]],
				intermediates: vec![b'?'],
				code: 'h',
			}]
		);

		// Control characters mixed with escapes
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x07\x1B[31m\x08"),
			vec![
				AnsiNode::ControlChar(0x07),
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::ControlChar(0x08),
			]
		);

		// Malformed escape that looks like ESC but isn't a sequence
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1BZ"),
			vec![AnsiNode::Esc {
				intermediates: vec![],
				byte: b'Z',
			}]
		);
	}

	#[test]
	fn real_world_sequences_test() {
		// Git diff colors
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[1;32m+added line\x1B[m"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![1], vec![32]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("+added line")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);

		// Prompt with multiple styles
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[1;34muser\x1B[0m@\x1B[1;32mhost\x1B[0m:"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![1], vec![34]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("user")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("@")),
				AnsiNode::Csi {
					params: vec![vec![1], vec![32]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("host")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(":")),
			]
		);

		// 256 color with reset
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[38;5;196mRED\x1B[0m"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![38], vec![5], vec![196]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("RED")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);
	}

	#[test]
	fn utf8_handling_test() {
		// Basic UTF-8
		assert_eq!(
			TerminalOutputParser::parse_to_nodes("Hello 世界 🦀".as_bytes()),
			vec![AnsiNode::Text(String::from("Hello 世界 🦀"))],
		);

		// UTF-8 mixed with escapes
		assert_eq!(
			TerminalOutputParser::parse_to_nodes("🎨\x1B[31müß\x1B[0m🔴".as_bytes()),
			vec![
				AnsiNode::Text(String::from("🎨")),
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("üß")),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("🔴")),
			]
		);
	}

	#[test]
	fn very_long_input_test() {
		let long_text = "a".repeat(10_000);
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(&format!("\x1B[31m{}\x1B[0m", long_text).as_bytes()),
			vec![
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from(long_text)),
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);
	}

	#[test]
	fn nested_and_overlapping_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(
				b"\x1B]0;Title\x07\x1B[31mRed\x1B]8;;http://example.com\x07Link\x1B]8;;\x07\x1B[0m"
			),
			vec![
				AnsiNode::Osc {
					params: vec![vec![b'0'], b"Title".to_vec()],
					bell_terminated: true,
				},
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Text(String::from("Red")),
				AnsiNode::Osc {
					params: vec![vec![b'8'], vec![], b"http://example.com".to_vec()],
					bell_terminated: true,
				},
				AnsiNode::Text(String::from("Link")),
				AnsiNode::Osc {
					params: vec![vec![b'8'], vec![], vec![]],
					bell_terminated: true,
				},
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);
	}

	#[test]
	fn backspace_behavior_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"abc\x08def"),
			vec![
				AnsiNode::Text(String::from("abc")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("def")),
			]
		);
	}

	#[test]
	fn carriage_return_handling_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"line1\r\nline2\nline3\r"),
			vec![AnsiNode::Text(String::from("line1\nline2\nline3\r"))],
		);
	}

	#[test]
	fn invalid_utf8_in_osc_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B]0;Valid\xFFInvalid\x07"),
			vec![AnsiNode::Osc {
				params: vec![
					vec![b'0'],
					vec![
						b'V', b'a', b'l', b'i', b'd', 0xFF, b'I', b'n', b'v', b'a', b'l', b'i', b'd',
					],
				],
				bell_terminated: true,
			},],
		);
	}

	#[test]
	fn zero_width_sequences_test() {
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[0m\x1B[0m\x1B[0m"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);
	}

	#[test]
	fn cursor_movement_sequences_test() {
		// Clear screen and home
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[2J\x1B[H"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![2]],
					intermediates: vec![],
					code: 'J',
				},
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'H',
				},
			]
		);

		// Cursor save/restore
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[s\x1B[u"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 's',
				},
				AnsiNode::Csi {
					params: vec![vec![0]],
					intermediates: vec![],
					code: 'u',
				},
			]
		);
	}

	#[test]
	fn overstrike_patterns_test() {
		// Old-style bold (char + backspace + char)
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"H\x08He\x08el\x08ll\x08lo\x08o"),
			vec![
				AnsiNode::Text(String::from("H")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("He")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("el")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("ll")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("lo")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("o")),
			]
		);

		// Old-style underline (_\bchar)
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"_\x08H_\x08i"),
			vec![
				AnsiNode::Text(String::from("_")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("H_")),
				AnsiNode::ControlChar(0x08),
				AnsiNode::Text(String::from("i")),
			]
		);
	}

	#[test]
	fn incomplete_utf8_sequence_test() {
		// Incomplete UTF-8 should be handled gracefully
		// This is a truncated 3-byte UTF-8 sequence
		// The parser should handle this without panicking
		assert!(!TerminalOutputParser::parse_to_nodes(b"Hello \xE2\x9C").is_empty());
	}

	#[test]
	fn csi_parameter_count_limit_test() {
		// Test with way too many parameters
		let mut params = String::from("\x1B[");
		for i in 0..100 {
			params.push_str(&format!("{};", i));
		}
		params.push('m');

		let mut result = Vec::with_capacity(32);
		for i in 0..32 {
			result.push(vec![i]);
		}

		assert_eq!(
			TerminalOutputParser::parse_to_nodes(params.as_bytes()),
			vec![AnsiNode::Csi {
				params: result,
				intermediates: vec![],
				code: 'm',
			}]
		);
	}

	#[test]
	fn csi_parameter_overflow_test() {
		// Test with parameter values at u16::MAX
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[65535;65535m"),
			vec![AnsiNode::Csi {
				params: vec![vec![65535], vec![65535]],
				intermediates: vec![],
				code: 'm',
			}]
		);
	}

	#[test]
	fn zero_width_style_sequences_test() {
		// Multiple style changes with no text should all be preserved
		assert_eq!(
			TerminalOutputParser::parse_to_nodes(b"\x1B[31m\x1B[1m\x1B[4m"),
			vec![
				AnsiNode::Csi {
					params: vec![vec![31]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![1]],
					intermediates: vec![],
					code: 'm',
				},
				AnsiNode::Csi {
					params: vec![vec![4]],
					intermediates: vec![],
					code: 'm',
				},
			]
		);
	}
}
