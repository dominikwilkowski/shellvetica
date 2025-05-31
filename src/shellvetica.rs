// TODO: add BgColors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
	Black,
	Red,
	Green,
	Yellow,
	Blue,
	Magenta,
	Cyan,
	White,
}

impl std::fmt::Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Color::Black => write!(f, "black"),
			Color::Red => write!(f, "red"),
			Color::Green => write!(f, "green"),
			Color::Yellow => write!(f, "yellow"),
			Color::Blue => write!(f, "blue"),
			Color::Magenta => write!(f, "magenta"),
			Color::Cyan => write!(f, "cyan"),
			Color::White => write!(f, "white"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
	Text(char),
	Color(Color),
	Close,
}

impl std::fmt::Display for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Token::Text(c) => write!(f, "{c}"),
			Token::Color(color) => write!(f, "<span style=\"color:{color}\">"),
			Token::Close => write!(f, "</span>"),
		}
	}
}

pub struct Shellvetica {
	ast: Vec<Token>,
}

impl Shellvetica {
	pub fn convert(input: &str) -> Self {
		Self {
			ast: Self::optimize_ast(&Self::str_2_ast(input)),
		}
	}

	pub fn str_2_ast(input: &str) -> Vec<Token> {
		let mut result = Vec::new();
		let mut chars = input.chars().peekable();

		while let Some(c) = chars.next() {
			match c {
				'\x1b' => {
					if let Some(&'[') = chars.peek() {
						chars.next();
						let mut sequence = String::from("\x1b[");

						while let Some(&next_char) = chars.peek() {
							sequence.push(chars.next().unwrap());

							if next_char.is_ascii_alphabetic() {
								break;
							}
						}

						let token = match sequence.as_str() {
							"\x1b[30m" => Token::Color(Color::Black),
							"\x1b[31m" => Token::Color(Color::Red),
							"\x1b[32m" => Token::Color(Color::Green),
							"\x1b[33m" => Token::Color(Color::Yellow),
							"\x1b[34m" => Token::Color(Color::Blue),
							"\x1b[35m" => Token::Color(Color::Magenta),
							"\x1b[36m" => Token::Color(Color::Cyan),
							"\x1b[37m" => Token::Color(Color::White),

							"\x1b[39m" | "\x1b[49m" | "\x1b[39;49m" | "\x1b[49;39m" | "\x1b[0m" => Token::Close,
							_ => Token::Color(Color::Black),
						};

						result.push(token);
					} else {
						result.push(Token::Text(c));
					}
				},
				_ => {
					result.push(Token::Text(c));
				},
			}
		}

		result
	}

	fn optimize_ast(ast: &Vec<Token>) -> Vec<Token> {
		let mut result = Vec::with_capacity(ast.len());
		let mut current_color = None;
		let mut i = 0;

		while i < ast.len() {
			match ast[i] {
				Token::Color(color) => {
					if let Some(open_color) = current_color {
						if open_color != color {
							current_color = Some(color);
							result.push(Token::Color(color));
						}
					} else {
						current_color = Some(color);
						result.push(Token::Color(color));
					}
					i += 1;
				},
				Token::Close => {
					if let Some(open_color) = current_color {
						let mut has_non_whitespace = false;
						let mut has_different_color = false;
						let mut has_color = false;
						let mut j = i + 1;

						while j < ast.len() {
							match &ast[j] {
								Token::Text(c) => {
									if !c.is_whitespace() {
										has_non_whitespace = true;
									}
									j += 1;
								},
								Token::Color(next_color) => {
									has_color = true;
									if *next_color != open_color {
										has_different_color = true;
									}
									break;
								},
								Token::Close => {
									has_non_whitespace = true;
									has_color = false;
									break;
								},
							}
						}

						if has_non_whitespace && has_color || has_different_color || j == ast.len() && !has_color {
							result.push(Token::Close);
							current_color = None;
						}
					}
					i += 1;
				},
				Token::Text(c) => {
					result.push(Token::Text(c));
					i += 1;
				},
			}
		}

		result
	}

	pub fn export(&self) -> String {
		self.ast.iter().map(|token| token.to_string()).collect::<String>()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn str_2_ast_test() {
		assert_eq!(
			Shellvetica::str_2_ast("test"),
			vec![Token::Text('t'), Token::Text('e'), Token::Text('s'), Token::Text('t')],
		);

		assert_eq!(
			Shellvetica::str_2_ast("test\x1B[0m"),
			vec![
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[39;49mt\x1B[49;39me\x1B[49mst\x1B[39m"),
			vec![
				Token::Close,
				Token::Text('t'),
				Token::Close,
				Token::Text('e'),
				Token::Close,
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[30mtest\x1B[0m"),
			vec![
				Token::Color(Color::Black),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[31mtest\x1B[39m"),
			vec![
				Token::Color(Color::Red),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[32mtest\x1B[39m"),
			vec![
				Token::Color(Color::Green),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[33mtest\x1B[39m"),
			vec![
				Token::Color(Color::Yellow),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[34mtest\x1B[39m"),
			vec![
				Token::Color(Color::Blue),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[35mtest\x1B[39m"),
			vec![
				Token::Color(Color::Magenta),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[36mtest\x1B[39m"),
			vec![
				Token::Color(Color::Cyan),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);

		assert_eq!(
			Shellvetica::str_2_ast("\x1B[37mtest\x1B[39m"),
			vec![
				Token::Color(Color::White),
				Token::Text('t'),
				Token::Text('e'),
				Token::Text('s'),
				Token::Text('t'),
				Token::Close,
			],
		);
	}

	#[test]
	fn optimize_ast_test() {
		assert_eq!(
			Shellvetica::optimize_ast(&vec![Token::Text('t'), Token::Text('e'), Token::Text('s'), Token::Text('t'),]),
			vec![Token::Text('t'), Token::Text('e'), Token::Text('s'), Token::Text('t'),]
		);
	}

	#[test]
	fn optimize_ast_unused_close_test() {
		assert_eq!(
			Shellvetica::optimize_ast(&vec![Token::Text('A'), Token::Close, Token::Text('B')]),
			vec![Token::Text('A'), Token::Text('B')]
		);

		assert_eq!(
			Shellvetica::optimize_ast(&vec![
				Token::Text('A'),
				Token::Close,
				Token::Close,
				Token::Close,
				Token::Text('B'),
			]),
			vec![Token::Text('A'), Token::Text('B')]
		);
	}

	#[test]
	fn optimize_ast_too_many_close_test() {
		assert_eq!(
			Shellvetica::optimize_ast(&vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Close,
				Token::Close,
				Token::Close,
				Token::Text('B'),
			]),
			vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Close,
				Token::Text('B'),
			]
		);
	}

	#[test]
	fn optimize_ast_whitespace_test() {
		assert_eq!(
			Shellvetica::optimize_ast(&vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Close,
				Token::Close,
				Token::Close,
				Token::Text(' '),
				Token::Text(' '),
				Token::Text(' '),
				Token::Color(Color::Red),
				Token::Text('B'),
				Token::Close,
				Token::Close,
			]),
			vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Text(' '),
				Token::Text(' '),
				Token::Text(' '),
				Token::Text('B'),
				Token::Close,
			]
		);

		assert_eq!(
			Shellvetica::optimize_ast(&vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Close,
				Token::Close,
				Token::Close,
				Token::Text(' '),
				Token::Text('X'),
				Token::Text(' '),
				Token::Color(Color::Red),
				Token::Text('B'),
				Token::Close,
				Token::Close,
			]),
			vec![
				Token::Color(Color::Red),
				Token::Text('A'),
				Token::Close,
				Token::Text(' '),
				Token::Text('X'),
				Token::Text(' '),
				Token::Color(Color::Red),
				Token::Text('B'),
				Token::Close,
			]
		);
	}

	#[test]
	fn optimize_ast_overwritten_colors_test() {
		assert_eq!(
			Shellvetica::optimize_ast(&vec![
				Token::Color(Color::Red),
				Token::Color(Color::Blue),
				Token::Text('A'),
				Token::Close,
				Token::Text('B'),
			]),
			vec![
				Token::Color(Color::Blue),
				Token::Text('A'),
				Token::Close,
				Token::Text('B'),
			]
		);
	}
}
