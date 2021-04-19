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

use unicode_segmentation::UnicodeSegmentation;

pub fn grapheme_truncate(input: &mut String, limit: usize, ellipse: char) {
    match UnicodeSegmentation::grapheme_indices(input.as_str(), true).nth(limit) {
        Some((i, _)) => {
            input.truncate(i);
            input.push(ellipse);
        }
        None => (),
    }
}
