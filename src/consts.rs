
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
  -p, --plain           Use simpler block chars to draw the clock.
  -f, --fancy           Use fancy clock style.
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

