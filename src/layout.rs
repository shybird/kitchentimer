use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::Config;
use crate::consts::*;

// If screen size falls below these values we skip computation of new
// positions.
const MIN_WIDTH: u16 = DIGIT_WIDTH * 6 + 13;
const MIN_HEIGHT: u16 = DIGIT_HEIGHT + 2;

pub struct Position {
    pub line: u16,
    pub col: u16,
}

pub struct Layout {
    pub force_redraw: bool, // Redraw elements on screen.
    pub force_recalc: Arc<AtomicBool>, // Recalculate position of elements.
    pub plain: bool, // Plain style clock.
    pub width: u16,
    pub height: u16,
    pub clock_sec: Position,
    pub clock_colon0: Position,
    pub clock_min: Position,
    pub clock_colon1: Position,
    pub clock_hr: Position,
    pub clock_days: Position,
    pub roster: Position,
    pub roster_width: u16,
    pub roster_height: u16,
    pub buffer: Position,
    pub error: Position,
    pub cursor: Position,
}

impl Layout {
    pub fn new(config: &Config) -> Layout {
        Layout {
            force_redraw: true,
            // May be set by signal handler (SIGWINCH).
            force_recalc: Arc::new(AtomicBool::new(true)),
            plain: config.plain,
            width: 0,
            height: 0,
            clock_sec: Position {col: 0, line: 0},
            clock_colon0: Position {col: 0, line: 0},
            clock_min: Position {col: 0, line: 0},
            clock_colon1: Position {col: 0, line: 0},
            clock_hr: Position {col: 0, line: 0},
            clock_days: Position {col: 0, line: 0},
            roster: Position {col: 1, line: 3},
            roster_width: 0,
            roster_height: 0,
            buffer: Position {col: 0, line: 0},
            error: Position {col: 0, line: 0},
            cursor: Position {col: 1, line: 1},
        }
    }

    pub fn update(&mut self, display_hours: bool, force: bool) {
        if self.force_recalc.swap(false, Ordering::Relaxed) || force {
            let (width, height) = termion::terminal_size()
                .expect("Could not read terminal size!");
            self.width = width;
            self.height = height;
            self.compute(display_hours);
            self.force_redraw = true;
        }
    }

    #[cfg(test)]
    pub fn test_update(
        &mut self,
        hours: bool,
        width: u16,
        height: u16,
        roster_width: u16,
    ) {
        self.width = width;
        self.height = height;
        self.roster_width = roster_width;
        self.compute(hours);
    }

    pub fn can_hold(&self, other: &str) -> bool {
        // Only valid for ascii strings.
        self.width >= other.len() as u16
    }

    // Compute the position of various elements based on the size of the
    // terminal.
    fn compute(&mut self, display_hours: bool) {
        // Prevent integer overflow at very low screen sizes.
        if self.width < MIN_WIDTH || self.height < MIN_HEIGHT { return; }

        let middle: u16 = self.height / 2 - 1;

        if display_hours {
            // Seconds digits.
            self.clock_sec.col = (self.width + self.roster_width) / 2 + DIGIT_WIDTH + 6;
            // Colon separating minutes from seconds.
            self.clock_colon0.col = (self.width + self.roster_width) / 2 + DIGIT_WIDTH + 3;
            // Minute digits.
            self.clock_min.col = (self.width + self.roster_width) / 2 - DIGIT_WIDTH;

            // Colon separating hours from minutes.
            self.clock_colon1 = Position {
                col: (self.width + self.roster_width) / 2 - (DIGIT_WIDTH + 3),
                line: middle,
            };

            // Hour digits.
            self.clock_hr = Position {
                col: (self.width + self.roster_width) / 2 - (DIGIT_WIDTH * 3 + 6),
                line: middle,
            };
        } else {
            // Seconds digits.
            self.clock_sec.col = (self.width + self.roster_width) / 2 + 3;
            // Colon separating minutes from seconds.
            self.clock_colon0.col = (self.width + self.roster_width) / 2;
            // Minute digits.
            self.clock_min.col = (self.width + self.roster_width) / 2 - (DIGIT_WIDTH * 2 + 3);
        }

        self.clock_sec.line = middle;
        self.clock_colon0.line = middle;
        self.clock_min.line = middle;

        // Days (based on position of seconds).
        self.clock_days = Position {
            line: self.clock_sec.line + DIGIT_HEIGHT,
            col: self.clock_sec.col,
        };

        // Alarm roster height.
        self.roster_height = self.height - self.roster.line - 1;

        // Input buffer.
        self.buffer = Position {
            line: self.height,
            col: 1,
        };

        // Error messages.
        self.error = Position {
            line: self.height,
            col: 12,
        };

        // Cursor. Column will be set by main loop.
        self.cursor.line = self.buffer.line;
    }

    pub fn set_roster_width(&mut self, width: u16) {
        if self.width != width {
            self.roster_width = width;
            self.force_recalc.store(true, Ordering::Relaxed);
        }
    }
}

