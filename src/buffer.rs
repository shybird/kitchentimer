extern crate unicode_segmentation;

use std::io::Write;
use termion::{clear, cursor, color};
use termion::raw::RawTerminal;
use crate::layout::Layout;
use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

const PROMPT: &str = "Add alarm: ";


// Input buffer.
pub struct Buffer {
    content: String,
    // Used for error messages.
    message: Option<&'static str>,
    pub visible: bool,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            content: String::new(),
            message: None,
            visible: false,
        }
    }

    // Return reference to buffer content.
    pub fn read(&mut self) -> &String {
        &self.content
    }

    // Append char to buffer.
    pub fn push(&mut self, value: char) {
        // Reset error message.
        self.message = None;
        self.content.push(value);
    }

    // Remove last char.
    pub fn strip_char(&mut self) {
        // Reset error message.
        self.message = None;
        self.content.pop();
    }

    // Remove last word.
    pub fn strip_word(&mut self) {
        // Reset error message.
        self.message = None;
        let iter = UnicodeSegmentation::split_word_bound_indices(
            self.content.as_str().trim_end());

        if let Some((index, _)) = iter.last() {
            self.content.truncate(index);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    // Clear input.
    pub fn clear(&mut self) {
        self.content.clear();
    }

    // Clear input and message.
    pub fn reset(&mut self) {
        self.message = None;
        self.clear();
    }

    // Draw input buffer.
    pub fn draw<W: Write>(
        &mut self,
        stdout: &mut RawTerminal<W>,
        layout: &mut Layout,
    ) -> Result<(), std::io::Error>
    {
        // Write error message if present and return.
        if let Some(msg) = self.message {
            write!(stdout,
                "{}{}{}{}{}{}{}",
                cursor::Hide,
                cursor::Goto( layout.buffer.col, layout.buffer.line),
                clear::CurrentLine,
                PROMPT,
                color::Fg(color::LightRed),
                &msg,
                color::Fg(color::Reset))?;
            return Ok(());
        }

        if self.content.is_empty() {
            // Clear buffer display.
            write!(stdout,
                "{}{}{}",
                cursor::Goto(layout.buffer.col, layout.buffer.line),
                clear::CurrentLine,
                cursor::Hide)?;
        } else {
            // Check if buffer exceeds limits.
            while UnicodeWidthStr::width(self.content.as_str())
                + UnicodeWidthStr::width(PROMPT)
                > layout.width as usize
            {
                self.content.pop();
            }

            write!(stdout,
                "{}{}{}{}{}",
                cursor::Goto(layout.buffer.col, layout.buffer.line),
                clear::CurrentLine,
                PROMPT,
                cursor::Show,
                &self.content)?;
        }
        Ok(())
    }

    // Draw error message at input buffer position.
    pub fn message(
        &mut self,
        msg: &'static str,
    ) {
        self.message = Some(msg);
    }
}
