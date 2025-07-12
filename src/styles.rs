#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
	/// Standard 8 colors (30-37, 40-47)
	Standard(u8),
	/// Bright variants of standard colors (with bright)
	Bright(u8),
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
	Default,
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
			0 => Some(Font::Default),
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
						0 => None, // Actually this should set underline to None
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
				[25, ..] => result.blink = false,
				[26, ..] => result.proportional_spacing = true,
				[27, ..] => result.reverse = false,
				[28, ..] => result.hidden = false,
				[29, ..] => result.strikethrough = false,

				// Standard foreground colors
				[n @ 30..=37, ..] => {
					let color_index = (n - 30) as u8;
					result.foreground = Some(if result.bold {
						Color::Bright(color_index)
					} else {
						Color::Standard(color_index)
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
						Color::Bright(color_index)
					} else {
						Color::Standard(color_index)
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

				// Bright foreground colors (direct)
				[n @ 90..=97, ..] => {
					result.foreground = Some(Color::Bright((n - 90) as u8));
				},

				// Bright background colors (direct)
				[n @ 100..=107, ..] => {
					result.background = Some(Color::Bright((n - 100) as u8));
				},

				_ => {}, // Unknown SGR code, ignore
			}
		}

		result
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
				foreground: Some(Color::Standard(1)),
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
				foreground: Some(Color::Standard(0)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![31]]),
			StyleNode {
				foreground: Some(Color::Standard(1)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![32]]),
			StyleNode {
				foreground: Some(Color::Standard(2)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![33]]),
			StyleNode {
				foreground: Some(Color::Standard(3)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![34]]),
			StyleNode {
				foreground: Some(Color::Standard(4)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![35]]),
			StyleNode {
				foreground: Some(Color::Standard(5)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![36]]),
			StyleNode {
				foreground: Some(Color::Standard(6)),
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![37]]),
			StyleNode {
				foreground: Some(Color::Standard(7)),
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
				foreground: Some(Color::Bright(0)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![31], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(1)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![32], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(2)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![33], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(3)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![34], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(4)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![35], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(5)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![36], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(6)),
				fg_bright_from_bold: true,
				..StyleNode::default()
			},
		);
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![37], vec![1]]),
			StyleNode {
				bold: true,
				foreground: Some(Color::Bright(7)),
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
				foreground: Some(Color::Bright(1)),
				..StyleNode::default()
			}
		);

		// Explicit bright should stay bright even after removing bold
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![91]]),
			StyleNode {
				foreground: Some(Color::Bright(1)),
				..StyleNode::default()
			}
		);

		// Apply and remove bold - explicit bright should remain
		assert_eq!(
			StyleNode::from_ansi_node(&[vec![91], vec![1], vec![22]]),
			StyleNode {
				foreground: Some(Color::Bright(1)),
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
}
