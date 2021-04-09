use unicode_segmentation::UnicodeSegmentation;
use termion::color;


pub struct Config {
    pub plain: bool,
    pub quit: bool,
    pub command: Option<Vec<String>>,
}

pub fn unicode_length(input: &str) -> u16 {
    let length = UnicodeSegmentation::graphemes(input, true).count();
    length as u16
}

pub fn unicode_truncate(input: &mut String, limit: usize) {
    match UnicodeSegmentation::grapheme_indices(input.as_str(), true).nth(limit) {
        Some((i, _)) => input.truncate(i),
        None => (),
    }
}

pub const COLOR: [&dyn color::Color; 6] = [
    &color::Cyan,
    &color::Magenta,
    &color::Green,
    &color::Yellow,
    &color::Blue,
    &color::Red,
];
// Maximum length of labels.
pub const LABEL_SIZE_LIMIT: usize = 48;
pub const DIGIT_HEIGHT: u16 = 5;
pub const DIGIT_WIDTH: u16 = 5;
pub const DIGITS: [[&str; DIGIT_HEIGHT as usize]; 10] = [
    [
    // 0
    "█▀▀▀█",
    "█   █",
    "█   █",
    "█   █",
    "█▄▄▄█",
    ], [
    // 1
    "  ▀█ ",
    "   █ ",
    "   █ ",
    "   █ ",
    "   █ "
    ], [
    // 2
    "▀▀▀▀█",
    "    █",
    "█▀▀▀▀",
    "█    ",
    "█▄▄▄▄"
    ], [
    // 3
    "▀▀▀▀█",
    "    █",
    " ▀▀▀█",
    "    █",
    "▄▄▄▄█"
    ], [
    // 4
    "█    ",
    "█  █ ",
    "▀▀▀█▀",
    "   █ ",
    "   █ "
    ], [
    // 5
    "█▀▀▀▀",
    "█    ",
    "▀▀▀▀█",
    "    █",
    "▄▄▄▄█"
    ], [
    // 6
    "█    ",
    "█    ",
    "█▀▀▀█",
    "█   █",
    "█▄▄▄█"
    ], [
    // 7
    "▀▀▀▀█",
    "    █",
    "   █ ",
    "  █  ",
    "  █  ",
    ], [
    // 8
    "█▀▀▀█",
    "█   █",
    "█▀▀▀█",
    "█   █",
    "█▄▄▄█"
    ], [
    // 9
    "█▀▀▀█",
    "█   █",
    "▀▀▀▀█",
    "    █",
    "    █"
    ]
];

pub const DIGITS_PLAIN: [[&str; DIGIT_HEIGHT as usize]; 10] = [
    [
    // 0
    "█████",
    "█   █",
    "█   █",
    "█   █",
    "█████"
    ], [
    // 1
    "  ██ ",
    "   █ ",
    "   █ ",
    "   █ ",
    "   █ "
    ], [
    // 2
    "█████",
    "    █",
    "█████",
    "█    ",
    "█████"
    ], [
    // 3
    "█████",
    "    █",
    " ████",
    "    █",
    "█████"
    ], [
    // 4
    "█    ",
    "█  █ ",
    "█████",
    "   █ ",
    "   █ "
    ], [
    // 5
    "█████",
    "█    ",
    "█████",
    "    █",
    "█████"
    ], [
    // 6
    "█    ",
    "█    ",
    "█████",
    "█   █",
    "█████"
    ], [
    // 7
    "█████",
    "    █",
    "   █ ",
    "  █  ",
    "  █  "
    ], [
    // 8
    "█████",
    "█   █",
    "█████",
    "█   █",
    "█████"
    ], [
    // 9
    "█████",
    "█   █",
    "█████",
    "    █",
    "    █"
    ]
];

