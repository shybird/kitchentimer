use crate::layout::Layout;
use crate::clock::Clock;
use crate::Config;

fn default_config() -> Config {
    Config {
        plain: false,
        quit: false,
        command: None,
    }
}

// Test if layout computation works without panicking.
#[test]
fn layout_computation() {
    let config = default_config();
    let mut clock = Clock::new(&config);
    let mut layout = Layout::new(&config);

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
