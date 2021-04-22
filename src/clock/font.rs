// Copyright 2021, Shy.
//
// This file is part of Kitchentimer.
//
// Kitchentimer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Kitchentimer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Kitchentimer.  If not, see <https://www.gnu.org/licenses/>.

const DIGIT_HEIGHT: u16 = 5;

pub struct Font {
    pub height: u16,
    pub width: u16,
    pub dots: (char, char),
    pub digits: [[&'static str; DIGIT_HEIGHT as usize]; 10],
}

pub const NORMAL: Font = Font {
    height: DIGIT_HEIGHT,
    width: 5,
    dots: ('■', '■'),
    digits: [
        [
            // 0
            "█▀▀▀█",
            "█   █",
            "█   █",
            "█   █",
            "█▄▄▄█",
        ],
        [
            // 1
            "  ▀█ ",
            "   █ ",
            "   █ ",
            "   █ ",
            "   █ ",
        ],
        [
            // 2
            "▀▀▀▀█",
            "    █",
            "█▀▀▀▀",
            "█    ",
            "█▄▄▄▄",
        ],
        [
            // 3
            "▀▀▀▀█",
            "    █",
            " ▀▀▀█",
            "    █",
            "▄▄▄▄█",
        ],
        [
            // 4
            "█    ",
            "█  █ ",
            "▀▀▀█▀",
            "   █ ",
            "   █ ",
        ],
        [
            // 5
            "█▀▀▀▀",
            "█    ",
            "▀▀▀▀█",
            "    █",
            "▄▄▄▄█",
        ],
        [
            // 6
            "█    ",
            "█    ",
            "█▀▀▀█",
            "█   █",
            "█▄▄▄█",
        ],
        [
            // 7
            "▀▀▀▀█",
            "    █",
            "   █ ",
            "  █  ",
            "  █  ",
        ],
        [
            // 8
            "█▀▀▀█",
            "█   █",
            "█▀▀▀█",
            "█   █",
            "█▄▄▄█",
        ],
        [
            // 9
            "█▀▀▀█",
            "█   █",
            "▀▀▀▀█",
            "    █",
            "    █",
        ],
    ],
};

pub const PLAIN: Font = Font {
    height: DIGIT_HEIGHT,
    width: 5,
    dots: ('█', '█'),
    digits: [
        [
            // 0
            "█████",
            "█   █",
            "█   █",
            "█   █",
            "█████",
        ],
        [
            // 1
            "  ██ ",
            "   █ ",
            "   █ ",
            "   █ ",
            "   █ ",
        ],
        [
            // 2
            "█████",
            "    █",
            "█████",
            "█    ",
            "█████",
        ],
        [
            // 3
            "█████",
            "    █",
            " ████",
            "    █",
            "█████",
        ],
        [
            // 4
            "█    ",
            "█  █ ",
            "█████",
            "   █ ",
            "   █ ",
        ],
        [
            // 5
            "█████",
            "█    ",
            "█████",
            "    █",
            "█████",
        ],
        [
            // 6
            "█    ",
            "█    ",
            "█████",
            "█   █",
            "█████",
        ],
        [
            // 7
            "█████",
            "    █",
            "   █ ",
            "  █  ",
            "  █  ",
        ],
        [
            // 8
            "█████",
            "█   █",
            "█████",
            "█   █",
            "█████",
        ],
        [
            // 9
            "█████",
            "█   █",
            "█████",
            "    █",
            "    █",
        ],
    ],
};

/*
pub const CHROME: Font = Font {
    height: DIGIT_HEIGHT,
    width: 5,
    dots: ('▄', '🮏'),
    digits: [
        [
            // 0
            "█▀▀▀█",
            "█   █",
            "▀   ▀",
            "🮐   🮐",
            "🮐🮏🮏🮏🮐",
        ],
        [
            // 1
            "  ▀█ ",
            "   █ ",
            "   ▀ ",
            "   🮐 ",
            "   🮐 ",
        ],
        [
            // 2
            "▀▀▀▀█",
            "    █",
            "▀▀▀▀▀",
            "🮐    ",
            "🮐🮏🮏🮏🮏",
        ],
        [
            // 3
            "▀▀▀▀█",
            "    █",
            " ▀▀▀▀",
            "    🮐",
            "🮏🮏🮏🮏🮐",
        ],
        [
            // 4
            "█    ",
            "█  █ ",
            "▀▀▀▀▀",
            "   🮐 ",
            "   🮐 ",
        ],
        [
            // 5
            "█▀▀▀▀",
            "█    ",
            "▀▀▀▀▀",
            "    🮐",
            "🮏🮏🮏🮏🮐",
        ],
        [
            // 6
            "█    ",
            "█    ",
            "▀▀▀▀▀",
            "🮐   🮐",
            "🮐🮏🮏🮏🮐",
        ],
        [
            // 7
            "▀▀▀▀█",
            "    █",
            "   ▀ ",
            "  🮐  ",
            "  🮐  ",
        ],
        [
            // 8
            "█▀▀▀█",
            "█   █",
            "▀▀▀▀▀",
            "🮐   🮐",
            "🮐🮏🮏🮏🮐",
        ],
        [
            // 9
            "█▀▀▀█",
            "█   █",
            "▀▀▀▀▀",
            "    🮐",
            "    🮐",
        ],
    ],
};
*/
