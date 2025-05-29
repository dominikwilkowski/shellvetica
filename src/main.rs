use std::io::{self, Read, stdin};

fn main() -> io::Result<()> {
	let mut buffer = Vec::new();
	stdin().read_to_end(&mut buffer)?;
	let input = String::from_utf8_lossy(&buffer);

	let mut ast = ansi2ast(&input);
	println!("{ast:?}\n");
	optimize_ast(&mut ast);
	println!("{ast:?}\n");
	let html = ast.iter().map(|token| token.to_string()).collect::<String>();
	println!("{html}");

	Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
	Blue,
	Yellow,
}

impl std::fmt::Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Color::Blue => write!(f, "blue"),
			Color::Yellow => write!(f, "yellow"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
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

fn ansi2ast(input: &str) -> Vec<Token> {
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
						"\x1b[34m" => Token::Color(Color::Blue),
						"\x1b[39m" | "\x1b[49m" | "\x1b[39;49m" | "\x1b[0m" => Token::Close,
						_ => Token::Color(Color::Blue),
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

fn optimize_ast(ast: &mut Vec<Token>) {
	let mut result = Vec::with_capacity(ast.len());
	let mut current_color = None;
	let mut i = 0;

	while i < ast.len() {
		match &ast[i] {
			Token::Color(color) => {
				if let Some(open_color) = current_color {
					if open_color != color {
						current_color = Some(color);
						result.push(Token::Color(*color));
					}
				} else {
					current_color = Some(color);
					result.push(Token::Color(*color));
				}
				i += 1;
			},
			Token::Close => {
				if let Some(open_color) = current_color {
					let mut has_non_whitespace = false;
					let mut has_different_color = false;
					let mut has_following_color = false;
					let mut j = i + 1;

					while j < ast.len() {
						match &ast[j] {
							Token::Text(c) => {
								if !c.is_whitespace() {
									has_non_whitespace = true;
									break;
								} else {
									j += 1;
								}
							},
							Token::Color(next_color) => {
								has_following_color = true;
								if next_color != open_color {
									has_different_color = true;
								}
								break;
							},
							Token::Close => {
								has_non_whitespace = true;
								break;
							},
						}
					}

					if !has_non_whitespace && has_different_color {
						result.push(Token::Close);
						current_color = None;
					}
				}
				i += 1;
			},
			Token::Text(c) => {
				result.push(Token::Text(*c));
				i += 1;
			},
		}
	}
	*ast = result;
}
