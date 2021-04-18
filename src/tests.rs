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

use crate::clock::{Clock, font};
use crate::layout::Layout;
use crate::Config;

fn default_config() -> Config {
    Config {
        fancy: false,
        quit: false,
        command: None,
        font: &font::NORMAL,
    }
}

// Test if layout computation works without panicking.
#[test]
fn layout_computation() {
    let config = default_config();
    let mut clock = Clock::new(&config);
    let mut layout = Layout::new();

    // Two segment display.
    for roster_width in &[0, 10, 20, 30, 40] {
        for width in 0..256 {
            for height in 0..128 {
                layout.test_update(&clock, width, height, *roster_width);
            }
        }
    }
    // Three segment display.
    clock.elapsed = 3600;
    for roster_width in &[0, 10, 20, 30, 40] {
        for width in 0..256 {
            for height in 0..128 {
                layout.test_update(&clock, width, height, *roster_width);
            }
        }
    }
}
