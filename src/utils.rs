extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

pub fn unicode_length(input: &str) -> u16 {
    let length = UnicodeSegmentation::graphemes(input, true).count();
    length as u16
}

pub fn unicode_truncate(input: &mut String, limit: usize) {
    match UnicodeSegmentation::grapheme_indices(input.as_str(), true).nth(limit) {
        Some((i, _)) => {
            input.truncate(i);
            input.push('…');
        },
        None => (),
    }
}

