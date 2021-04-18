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

pub const COLOR: [&dyn termion::color::Color; 6] = [
    &termion::color::LightGreen,
    &termion::color::LightYellow,
    &termion::color::LightMagenta,
    &termion::color::LightCyan,
    &termion::color::LightRed,
    &termion::color::LightBlue,
];

// Maximum length of labels.
pub const LABEL_SIZE_LIMIT: usize = 32;

pub mod ui {
    pub const NAME: &str = env!("CARGO_PKG_NAME");
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const USAGE: &str = concat!(
        "USAGE: ",
        env!("CARGO_PKG_NAME"),
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
  -p, --plain           Use simpler block chars to draw the clock.
  -f, --fancy           Make use of less common unicode characters.
  -q, --quit            Quit program after last alarm.

SIGNALS: <SIGUSR1> Reset clock.
         <SIGUSR2> Pause or un-pause clock."
    );
    pub const MENUBAR: &str =
        "[0-9] Add alarm  [d] Delete alarm  [SPACE] Pause  [r] Reset  [c] Clear color  [q] Quit";
    pub const MENUBAR_SHORT: &str =
        "[0-9] Add  [d] Delete  [SPACE] Pause  [r] Reset  [c] Clear  [q] Quit";
    pub const MENUBAR_INS: &str =
        "Format: HH:MM:SS/LABEL  [ENTER] Accept  [ESC] Cancel  [CTR-C] Quit";
    pub const MENUBAR_PAUSED: &str = "[SPACE] Continue  [r] Reset  [UP]/[DOWN] Set clock";
}
