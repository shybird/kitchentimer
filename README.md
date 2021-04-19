# kitchentimer

Simple timing application for text terminals. Helper for preparing tea,
coffee, pasta etc.
Features a single large up-counting clock with optional alarm times. Alarm
times may be given as command line arguments, read from stdin or entered
interactively.

## Installation

    cargo install --git "https://git.twix.ws/shy/kitchentimer.git"

## Usage

    USAGE: kitchentimer [-h|-v] [-e|--exec COMMAND] [-p] [-q] [ALARM[/LABEL]]

    PARAMETERS:
      [ALARM TIME[/LABEL]]  Any number of alarm times (HH:MM:SS) with optional
                            label.

    OPTIONS:
      -h, --help            Show this usage message.
      -v, --version         Show version information.
      -e, --exec [COMMAND]  Execute COMMAND on alarm. My be given multiple
                            times. Occurrences of {l} will be replaced by the
                            alarm label, {t} by alarm time in (HH:)MM:SS format.
      -p, --plain           Use simpler block chars to draw the clock.
      -f, --fancy           Make use of less common unicode characters.
      -q, --quit            Quit program after last alarm.

    SIGNALS: <SIGUSR1> Reset clock.
             <SIGUSR2> Pause or continue.

