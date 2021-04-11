
pub const COLOR: [&dyn termion::color::Color; 6] = [
    &termion::color::Cyan,
    &termion::color::Magenta,
    &termion::color::Green,
    &termion::color::Yellow,
    &termion::color::Blue,
    &termion::color::Red,
];

// Maximum length of labels.
pub const LABEL_SIZE_LIMIT: usize = 48;

pub mod ui {
    pub const NAME: &str = env!("CARGO_PKG_NAME");
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const USAGE: &str = concat!("USAGE: ", env!("CARGO_PKG_NAME"),
" [-h|-v] [-e|--exec COMMAND] [-p] [-q] [ALARM[/LABEL]]

PARAMETERS:
  [ALARM TIME[/LABEL]]  Any number of alarm times (HH:MM:SS) with optional
                        label.

OPTIONS:
  -h, --help            Show this help.
  -v, --version         Show version information.
  -e, --exec [COMMAND]  Execute COMMAND on alarm. Occurrences of {t} will
                        be replaced by the alarm time in (HH:)MM:SS format.
                        Occurrences of {l} by alarm label.
  -p, --plain           Use simpler block chars.
  -q, --quit            Quit program after last alarm.

SIGNALS: <SIGUSR1> Reset clock.
         <SIGUSR2> Pause or un-pause clock.");
    pub const MENUBAR: &str =
    "[0-9] Add alarm  [d] Delete alarm  [SPACE] Pause  [r] Reset  [c] Clear color  [q] Quit";
    pub const MENUBAR_SHORT: &str =
    "[0-9] Add  [d] Delete  [SPACE] Pause  [r] Reset  [c] Clear  [q] Quit";
    pub const MENUBAR_INS: &str =
    "Format: HH:MM:SS/LABEL  [ENTER] Accept  [ESC] Cancel  [CTR-C] Quit";
}

pub mod digits {
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

    pub const _DIGITS_FANCY: [[&str; DIGIT_HEIGHT as usize]; 10] = [
        [
        // 0
        "█▀▀▀█",
        "█   █",
        "▀   ▀",
        "🮐   🮐",
        "🮐🮏🮏🮏🮐",
        ], [
        // 1
        "  ▀█ ",
        "   █ ",
        "   ▀ ",
        "   🮐 ",
        "   🮐 "
        ], [
        // 2
        "▀▀▀▀█",
        "    █",
        "▀▀▀▀▀",
        "🮐    ",
        "🮐🮏🮏🮏🮏"
        ], [
        // 3
        "▀▀▀▀█",
        "    █",
        " ▀▀▀▀",
        "    🮐",
        "🮏🮏🮏🮏🮐"
        ], [
        // 4
        "█    ",
        "█  █ ",
        "▀▀▀▀▀",
        "   🮐 ",
        "   🮐 "
        ], [
        // 5
        "█▀▀▀▀",
        "█    ",
        "▀▀▀▀▀",
        "    🮐",
        "🮏🮏🮏🮏🮐"
        ], [
        // 6
        "█    ",
        "█    ",
        "▀▀▀▀▀",
        "🮐   🮐",
        "🮐🮏🮏🮏🮐"
        ], [
        // 7
        "▀▀▀▀█",
        "    █",
        "   ▀ ",
        "  🮐  ",
        "  🮐  ",
        ], [
        // 8
        "█▀▀▀█",
        "█   █",
        "▀▀▀▀▀",
        "🮐   🮐",
        "🮐🮏🮏🮏🮐"
        ], [
        // 9
        "█▀▀▀█",
        "█   █",
        "▀▀▀▀▀",
        "    🮐",
        "    🮐"
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
}

