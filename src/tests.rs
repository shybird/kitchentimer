use crate::layout::Layout;
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
    let mut layout = Layout::new(&config);

    for roster_width in &[0, 10, 20, 30, 40] {
        for width in 0..256 {
            for height in 0..128 {
                layout.test_update(height & 1 == 0, width, height, *roster_width);
            }
        } 
    }
}
