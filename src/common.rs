use unicode_segmentation::UnicodeSegmentation;
use termion::color;


pub struct Config {
    pub plain: bool,
    pub quit: bool,
    pub command: Option<Vec<String>>,
}

pub fn str_length(input: &str) -> u16 {
    let length = UnicodeSegmentation::graphemes(input, true).count();
    length as u16
}

pub const COLOR: [&dyn color::Color; 6] = [
    &color::Cyan,
    &color::Magenta,
    &color::Green,
    &color::Yellow,
    &color::Blue,
    &color::Red,
];
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

