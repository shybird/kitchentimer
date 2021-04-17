use crate::clock::Clock;

pub struct Position {
    pub line: u16,
    pub col: u16,
}

impl Position {
    // Terminal positions are 1-based.
    pub fn new() -> Position {
        Position { col: 1, line: 1 }
    }
}

pub struct Layout {
    pub force_redraw: bool, // Redraw elements on screen.
    force_recalc: bool,     // Recalculate position of elements.
    pub width: u16,
    pub height: u16,
    clock_width: u16,
    clock_height: u16,
    digit_width: u16,
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
}

impl Layout {
    pub fn new() -> Layout {
        Layout {
            force_redraw: true,
            force_recalc: true,
            width: 0,
            height: 0,
            clock_width: 0,
            clock_height: 0,
            digit_width: 0,
            clock_sec: Position::new(),
            clock_colon0: Position::new(),
            clock_min: Position::new(),
            clock_colon1: Position::new(),
            clock_hr: Position::new(),
            clock_days: Position::new(),
            roster: Position { col: 1, line: 3 },
            roster_width: 0,
            roster_height: 0,
            buffer: Position::new(),
        }
    }

    // Update layout. Returns true when changes were made.
    pub fn update(&mut self, clock: &Clock, force: bool) -> Result<bool, std::io::Error> {
        if self.force_recalc || force {
            self.force_recalc = false;
            let (width, height) = termion::terminal_size()?;
            self.width = width;
            self.height = height;
            self.clock_width = clock.get_width();
            self.clock_height = clock.font.height;
            self.digit_width = clock.font.width;
            self.compute(clock.elapsed >= 3600);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn schedule_recalc(&mut self) {
        self.force_recalc = true;
    }

    #[cfg(test)]
    pub fn test_update(&mut self, clock: &Clock, width: u16, height: u16, roster_width: u16) {
        self.width = width;
        self.height = height;
        self.clock_width = clock.get_width();
        self.clock_height = clock.font.height;
        self.digit_width = clock.font.width;
        self.roster_width = roster_width;
        self.compute(false);
    }

    pub fn can_hold(&self, other: &str) -> bool {
        // Only valid for ascii strings.
        self.width >= other.len() as u16
    }

    // Compute the position of various elements based on the size of the
    // terminal.
    fn compute(&mut self, display_hours: bool) {
        // Prevent integer overflow at very low screen sizes.
        if self.width < self.clock_width || self.height < self.clock_height {
            return;
        }

        let middle: u16 = self.height / 2 - 1;

        if display_hours {
            // Seconds digits.
            self.clock_sec.col = (self.width + self.roster_width) / 2 + self.digit_width + 6;
            // Colon separating minutes from seconds.
            self.clock_colon0.col = (self.width + self.roster_width) / 2 + self.digit_width + 3;
            // Minute digits.
            self.clock_min.col = (self.width + self.roster_width) / 2 - self.digit_width;

            // Colon separating hours from minutes.
            self.clock_colon1 = Position {
                col: (self.width + self.roster_width) / 2 - (self.digit_width + 3),
                line: middle,
            };

            // Hour digits.
            self.clock_hr = Position {
                col: (self.width + self.roster_width) / 2 - (self.digit_width * 3 + 6),
                line: middle,
            };
        } else {
            // Seconds digits.
            self.clock_sec.col = (self.width + self.roster_width) / 2 + 3;
            // Colon separating minutes from seconds.
            self.clock_colon0.col = (self.width + self.roster_width) / 2;
            // Minute digits.
            self.clock_min.col = (self.width + self.roster_width) / 2 - (self.digit_width * 2 + 3);
        }

        self.clock_sec.line = middle;
        self.clock_colon0.line = middle;
        self.clock_min.line = middle;

        // Days (based on position of seconds).
        self.clock_days = Position {
            line: self.clock_sec.line + self.digit_width,
            col: self.clock_sec.col,
        };

        // Alarm roster height.
        self.roster_height = self.height - self.roster.line - 1;

        // Input buffer.
        self.buffer = Position {
            line: self.height,
            col: 1,
        };
    }

    pub fn set_roster_width(&mut self, width: u16) {
        if self.width != width {
            self.roster_width = width;
            self.force_recalc = true;
        }
    }
}
