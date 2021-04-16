extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

pub fn grapheme_truncate(input: &mut String, limit: usize, ellipse: char) {
    match UnicodeSegmentation::grapheme_indices(input.as_str(), true).nth(limit) {
        Some((i, _)) => {
            input.truncate(i);
            input.push(ellipse);
        },
        None => (),
    }
}

