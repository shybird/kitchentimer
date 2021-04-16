extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

pub fn unicode_truncate(input: &mut String, limit: usize) {
    match UnicodeSegmentation::grapheme_indices(input.as_str(), true).nth(limit) {
        Some((i, _)) => {
            input.truncate(i);
            input.push('…');
        },
        None => (),
    }
}

