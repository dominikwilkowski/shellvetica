#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EightBitColor {
	Black,
	Red,
	Green,
	Yellow,
	Blue,
	Magenta,
	Cyan,
	White,
}

impl EightBitColor {
	pub fn from_u8(value: u8) -> Self {
		match value {
			0 => EightBitColor::Black,
			1 => EightBitColor::Red,
			2 => EightBitColor::Green,
			3 => EightBitColor::Yellow,
			4 => EightBitColor::Blue,
			5 => EightBitColor::Magenta,
			6 => EightBitColor::Cyan,
			7 => EightBitColor::White,
			_ => EightBitColor::Black,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
	/// Standard 8 colors (30-37, 40-47)
	Standard(EightBitColor),
	/// Bright variants of standard colors (with bright)
	Bright(EightBitColor),
	/// 256-color palette
	Palette(u8),
	/// True color RGB
	Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnderlineStyle {
	Single,
	Double,
	Curly,
	Dotted,
	Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Font {
	One,
	Two,
	Three,
	Four,
	Five,
	Six,
	Seven,
	Eight,
	Nine,
}

impl Font {
	fn from_u8(value: u8) -> Option<Self> {
		match value {
			0 => None,
			1 => Some(Font::One),
			2 => Some(Font::Two),
			3 => Some(Font::Three),
			4 => Some(Font::Four),
			5 => Some(Font::Five),
			6 => Some(Font::Six),
			7 => Some(Font::Seven),
			8 => Some(Font::Eight),
			9 => Some(Font::Nine),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StyleNode {
	bold: bool,
	dim: bool,
	italic: bool,
	underline: Option<UnderlineStyle>,
	underline_color: Option<Color>,
	subscript: bool,
	superscript: bool,
	blink: bool,
	reverse: bool,
	hidden: bool,
	strikethrough: bool,
	rapid_blink: bool,
	font: Option<Font>,
	fraktur: bool,
	proportional_spacing: bool,
	framed: bool,
	encircled: bool,
	overlined: bool,
	foreground: Option<Color>,
	background: Option<Color>,
	fg_bright_from_bold: bool,
	bg_bright_from_bold: bool,
}

impl StyleNode {
	const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

	pub fn from_ansi_node(params: &[Vec<u16>]) -> Self {
		let mut result = Self::default();

		for param_group in params {
			match param_group.as_slice() {
				// Reset all
				[0, ..] => result = Self::default(),

				// Styles
				[1, ..] => {
					result.bold = true;
					// If we already have a standard foreground color, upgrade it to bright
					if let Some(Color::Standard(n)) = result.foreground {
						result.foreground = Some(Color::Bright(n));
						result.fg_bright_from_bold = true;
					}
					// Upgrade standard background to bright
					if let Some(Color::Standard(n)) = result.background {
						result.background = Some(Color::Bright(n));
						result.bg_bright_from_bold = true;
					}
				},
				[2, ..] => result.dim = true,
				[3, ..] => result.italic = true,

				// Underline with style (4:2 becomes [4, 2])
				[4, style, ..] => {
					result.underline = match style {
						0 => None,
						1 => Some(UnderlineStyle::Single),
						2 => Some(UnderlineStyle::Double),
						3 => Some(UnderlineStyle::Curly),
						4 => Some(UnderlineStyle::Dotted),
						5 => Some(UnderlineStyle::Dashed),
						_ => Some(UnderlineStyle::Single),
					};
				},
				[4] => result.underline = Some(UnderlineStyle::Single),

				[5, ..] => result.blink = true,
				[6, ..] => result.rapid_blink = true,
				[7, ..] => result.reverse = true,
				[8, ..] => result.hidden = true,
				[9, ..] => result.strikethrough = true,
				[10, ..] => result.font = Font::from_u8(0),
				[n @ 11..=19, ..] => result.font = Font::from_u8((n - 10) as u8),
				[20, ..] => result.fraktur = true,

				// Reset individual attributes
				[21 | 22, ..] => {
					result.bold = false;
					result.dim = false;
					// Downgrade bright colors if they came from bold
					if result.fg_bright_from_bold {
						if let Some(Color::Bright(n)) = result.foreground {
							result.foreground = Some(Color::Standard(n));
							result.fg_bright_from_bold = false;
						}
					}
					if result.bg_bright_from_bold {
						if let Some(Color::Bright(n)) = result.background {
							result.background = Some(Color::Standard(n));
							result.bg_bright_from_bold = false;
						}
					}
				},
				[23, ..] => result.italic = false,
				[24, ..] => result.underline = None,
				[25, ..] => {
					result.blink = false;
					result.rapid_blink = false;
				},
				[26, ..] => result.proportional_spacing = true,
				[27, ..] => result.reverse = false,
				[28, ..] => result.hidden = false,
				[29, ..] => result.strikethrough = false,

				// Standard foreground colors
				[n @ 30..=37, ..] => {
					let color_index = (n - 30) as u8;
					result.foreground = Some(if result.bold {
						result.fg_bright_from_bold = true;
						Color::Bright(EightBitColor::from_u8(color_index))
					} else {
						Color::Standard(EightBitColor::from_u8(color_index))
					});
				},

				// Extended foreground colors
				[38, 5, palette, ..] => {
					result.foreground = Some(Color::Palette(*palette as u8));
				},
				[38, 2, r, g, b, ..] => {
					result.foreground = Some(Color::Rgb {
						r: (*r).min(255) as u8,
						g: (*g).min(255) as u8,
						b: (*b).min(255) as u8,
					});
				},

				// Default foreground
				[39, ..] => result.foreground = None,

				// Standard background colors
				[n @ 40..=47, ..] => {
					let color_index = (n - 40) as u8;
					result.background = Some(if result.bold {
						result.bg_bright_from_bold = true;
						Color::Bright(EightBitColor::from_u8(color_index))
					} else {
						Color::Standard(EightBitColor::from_u8(color_index))
					});
				},

				// Extended background colors
				[48, 5, palette, ..] => {
					result.background = Some(Color::Palette(*palette as u8));
				},
				[48, 2, r, g, b, ..] => {
					result.background = Some(Color::Rgb {
						r: (*r).min(255) as u8,
						g: (*g).min(255) as u8,
						b: (*b).min(255) as u8,
					});
				},

				// Default background
				[49, ..] => result.background = None,

				// Legacy styles
				[50, ..] => result.proportional_spacing = false,
				[51, ..] => result.framed = true,
				[52, ..] => result.encircled = true,
				[53, ..] => result.overlined = true,
				[54, ..] => {
					result.framed = false;
					result.encircled = false;
				},
				[55, ..] => result.overlined = false,

				// Extended underline colors
				[58, 5, palette, ..] => {
					result.underline_color = Some(Color::Palette(*palette as u8));
				},
				[58, 2, r, g, b, ..] => {
					result.underline_color = Some(Color::Rgb {
						r: (*r).min(255) as u8,
						g: (*g).min(255) as u8,
						b: (*b).min(255) as u8,
					});
				},
				[59, ..] => result.underline_color = None,

				// Sub/superscript
				[73, ..] => {
					result.superscript = true;
					result.subscript = false;
				},
				[74, ..] => {
					result.subscript = true;
					result.superscript = false;
				},
				[75, ..] => {
					result.subscript = false;
					result.superscript = false;
				},

				// Bright foreground colors (direct)
				[n @ 90..=97, ..] => {
					result.foreground = Some(Color::Bright(EightBitColor::from_u8((n - 90) as u8)));
				},

				// Bright background colors (direct)
				[n @ 100..=107, ..] => {
					result.background = Some(Color::Bright(EightBitColor::from_u8((n - 100) as u8)));
				},

				_ => {}, // Unknown SGR code, ignore
			}
		}

		result
	}

	fn standard_color_to_hex(color: &EightBitColor) -> &'static str {
		match color {
			EightBitColor::Black => "#000",
			EightBitColor::Red => "#cd0000",
			EightBitColor::Green => "#00cd00",
			EightBitColor::Yellow => "#cdcd00",
			EightBitColor::Blue => "#00e",
			EightBitColor::Magenta => "#cd00cd",
			EightBitColor::Cyan => "#00cdcd",
			EightBitColor::White => "#e5e5e5",
		}
	}

	fn bright_color_to_hex(color: &EightBitColor) -> &'static str {
		match color {
			EightBitColor::Black => "#7f7f7f",
			EightBitColor::Red => "#f00",
			EightBitColor::Green => "#0f0",
			EightBitColor::Yellow => "#ff0",
			EightBitColor::Blue => "#5c5cff",
			EightBitColor::Magenta => "#f0f",
			EightBitColor::Cyan => "#0ff",
			EightBitColor::White => "#fff",
		}
	}

	fn append_color(html: &mut String, color: &Color) {
		match color {
			Color::Standard(color) => {
				html.push_str(Self::standard_color_to_hex(&color));
			},
			Color::Bright(color) => {
				html.push_str(Self::bright_color_to_hex(&color));
			},
			Color::Palette(color) => match color {
				0..=7 => html.push_str(Self::standard_color_to_hex(&EightBitColor::from_u8(*color))),
				8..=15 => html.push_str(Self::bright_color_to_hex(&EightBitColor::from_u8(color - 8))),
				16..=231 => {
					let n = color - 16;
					let r = (n / 36) * 51;
					let g = ((n % 36) / 6) * 51;
					let b = (n % 6) * 51;
					Self::push_hex_rgb(html, r, g, b);
				},
				232..=255 => {
					let gray = 8 + (color - 232) * 10;
					Self::push_hex_rgb(html, gray, gray, gray);
				},
			},
			Color::Rgb { r, g, b } => {
				Self::push_hex_rgb(html, *r, *g, *b);
			},
		};
	}

	pub fn to_html(&mut self) -> String {
		let mut html = String::with_capacity(200);

		let tag = if self.subscript {
			"sub"
		} else if self.superscript {
			"sup"
		} else {
			"span"
		};

		html.push_str("<");
		html.push_str(tag);
		html.push_str(" style=\"");

		if self.bold {
			html.push_str("font-weight:bold;");
		}

		if self.dim {
			html.push_str("opacity:.5;");
		}

		if self.italic {
			html.push_str("font-style:italic;");
		}

		if let Some(underline) = self.underline {
			match underline {
				UnderlineStyle::Single => html.push_str("text-decoration:underline;"),
				UnderlineStyle::Double => html.push_str("text-decoration:underline double;"),
				UnderlineStyle::Curly => html.push_str("text-decoration:underline wavy;"),
				UnderlineStyle::Dotted => html.push_str("text-decoration:underline dotted;"),
				UnderlineStyle::Dashed => html.push_str("text-decoration:underline dashed;"),
			}
		}

		if let Some(underline_color) = self.underline_color {
			html.push_str("text-decoration-color:");
			Self::append_color(&mut html, &underline_color);
			html.push(';');
		}

		// blink
		// hidden
		// strikethrough
		// rapid_blink
		// font
		// fraktur
		// proportional_spacing
		// framed
		// encircled
		// overlined

		if self.reverse {
			let bg = self.background;
			self.background = self.foreground;
			self.foreground = bg;
		}

		if let Some(color) = self.foreground {
			html.push_str("color:");
			Self::append_color(&mut html, &color);
			html.push(';');
		}

		if let Some(color) = self.background {
			html.push_str("background:");
			Self::append_color(&mut html, &color);
			html.push(';');
		}

		html.push_str("\">");
		html
	}

	#[inline]
	fn push_hex(s: &mut String, byte: u8) {
		s.push(Self::HEX_CHARS[(byte >> 4) as usize] as char);
		s.push(Self::HEX_CHARS[(byte & 0xf) as usize] as char);
	}

	#[inline]
	fn push_hex_rgb(s: &mut String, r: u8, g: u8, b: u8) {
		s.push('#');

		if r & 0x0F == r >> 4 && g & 0x0F == g >> 4 && b & 0x0F == b >> 4 {
			// Can use shorthand #RGB
			s.push(Self::HEX_CHARS[(r & 0x0F) as usize] as char);
			s.push(Self::HEX_CHARS[(g & 0x0F) as usize] as char);
			s.push(Self::HEX_CHARS[(b & 0x0F) as usize] as char);
		} else {
			// Must use full #RRGGBB
			Self::push_hex(s, r);
			Self::push_hex(s, g);
			Self::push_hex(s, b);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn styles_convert_test() {
		assert_eq!(StyleNode::from_ansi_node(&[vec![0]]), StyleNode::default(),);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![1]]),
			StyleNode {
				bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![2]]),
			StyleNode {
				dim: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![3]]),
			StyleNode {
				italic: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4]]),
			StyleNode {
				underline: Some(UnderlineStyle::Single),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 0], vec![31]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 1]]),
			StyleNode {
				underline: Some(UnderlineStyle::Single),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 2]]),
			StyleNode {
				underline: Some(UnderlineStyle::Double),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 3]]),
			StyleNode {
				underline: Some(UnderlineStyle::Curly),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 4]]),
			StyleNode {
				underline: Some(UnderlineStyle::Dotted),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![4, 5]]),
			StyleNode {
				underline: Some(UnderlineStyle::Dashed),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![5]]),
			StyleNode {
				blink: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![7]]),
			StyleNode {
				reverse: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![8]]),
			StyleNode {
				hidden: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![9]]),
			StyleNode {
				strikethrough: true,
				..StyleNode::default()
			},
		);
	}

	#[test]
	fn standard_foreground_color_convert_test() {
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![30]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(0))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![31]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(1))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![32]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(2))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![33]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(3))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![34]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(4))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![35]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(5))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![36]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(6))),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![37]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(7))),
				..StyleNode::default()
			},
		);
	}

	#[test]
	fn bright_foreground_color_convert_test() {
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![30], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(0))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![31], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![32], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(2))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![33], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(3))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![34], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(4))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![35], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(5))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![36], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(6))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![37], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(7))),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
	}

	#[test]
	fn bold_bright_interaction_test() {
		// Bold then color should give bright
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![1], vec![31]]),
			StyleNode {
				bold: true,
				fg_bright_from_bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);

		// Color with bold modifier should give bright
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![31], vec![1]]),
			StyleNode {
				bold: true,
				fg_bright_from_bold: true,
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);

		// Explicit bright should stay bright even after removing bold
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![91]]),
			StyleNode {
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);

		// Apply and remove bold - explicit bright should remain
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![91], vec![1], vec![22]]),
			StyleNode {
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);
	}

	#[test]
	fn bold_then_color_then_reset_test() {
		// Apply bold, then color, then remove bold - color should downgrade
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![1], vec![31], vec![22]]),
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);

		// Compare with explicit bright color which should NOT downgrade
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![91], vec![22]]),
			StyleNode {
				foreground: Some(Color::Bright(EightBitColor::from_u8(1))),
				..StyleNode::default()
			}
		);
	}

	#[test]
	fn bold_after_256_color_test() {
		// Bold shouldn't affect 256 colors
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![38, 5, 196], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Palette(196)),
				..StyleNode::default()
			}
		);
	}

	#[test]
	fn bold_after_rgb_color_test() {
		// Bold shouldn't affect RGB colors
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![38, 2, 255, 0, 0], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Rgb { r: 255, g: 0, b: 0 }),
				..StyleNode::default()
			}
		);
	}

	#[test]
	fn sgr_22_with_bold_and_dim_test() {
		// SGR 22 should reset both bold and dim
		assert_eq!(StyleNode::from_ansi_node(&[vec![1], vec![2], vec![22]]), StyleNode::default());
	}

	#[test]
	fn sgr_22_preserves_other_attributes_test() {
		// SGR 22 should only affect bold/dim
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![1], vec![3], vec![4], vec![22]]),
			StyleNode {
				italic: true,
				underline: Some(UnderlineStyle::Single),
				..StyleNode::default()
			}
		);
	}

	#[test]
	fn underline_color_test() {
		// 256 color underline
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![58, 5, 196]]),
			StyleNode {
				underline_color: Some(Color::Palette(196)),
				..StyleNode::default()
			}
		);

		// RGB underline
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![58, 2, 255, 0, 128]]),
			StyleNode {
				underline_color: Some(Color::Rgb { r: 255, g: 0, b: 128 }),
				..StyleNode::default()
			}
		);

		// Reset underline color
		assert_eq!(StyleNode::from_ansi_node(&[vec![58, 5, 196], vec![59]]), StyleNode::default());
	}

	#[test]
	fn subscript_superscript_test() {
		// Superscript
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![73]]),
			StyleNode {
				superscript: true,
				..StyleNode::default()
			}
		);

		// Subscript
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![74]]),
			StyleNode {
				subscript: true,
				..StyleNode::default()
			}
		);

		// Superscript overrides subscript
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![74], vec![73]]),
			StyleNode {
				superscript: true,
				subscript: false,
				..StyleNode::default()
			}
		);

		// Reset both
		assert_eq!(StyleNode::from_ansi_node(&[vec![73], vec![75]]), StyleNode::default());
	}

	#[test]
	fn to_html_test() {
		// Standard colors
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Standard(EightBitColor::Black)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#000;\">")
		);

		// Bright colors
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Bright(EightBitColor::Blue)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#5c5cff;\">")
		);

		// Palette colors
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Palette(5)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#cd00cd;\">")
		);
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Palette(12)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#5c5cff;\">")
		);
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Palette(190)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#cf0;\">")
		);
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Palette(245)),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#8a8a8a;\">")
		);

		// RGB colors
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Rgb { r: 255, g: 0, b: 128 }),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#ff0080;\">")
		);
		assert_eq!(
			StyleNode {
				foreground: Some(Color::Rgb { r: 17, g: 34, b: 51 }),
				..StyleNode::default()
			}
			.to_html(),
			String::from("<span style=\"color:#123;\">")
		);
	}
}
