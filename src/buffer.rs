extern crate unicode_segmentation;

use std::io::Write;
use termion::{clear, cursor, color};
use termion::raw::RawTerminal;
use crate::layout::Layout;
use unicode_segmentation::UnicodeSegmentation;

const PROMPT: &str = "Add alarm: ";


// Input buffer.
pub struct Buffer {
    content: String,
    // Used for error messages.
    message: Option<&'static str>,
    pub altered: bool,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            content: String::new(),
            altered: false,
            message: None,
        }
    }

    // Return reference to buffer content.
    pub fn read(&mut self) -> &String {
        self.altered = false;
        &self.content
    }

    // Append char to buffer.
    pub fn push(&mut self, value: char) {
        self.altered = true;
        // Reset error message.
        self.message = None;
        self.content.push(value);
    }

    // Remove last char. Return true if a char was removed.
    pub fn strip_char(&mut self) -> bool {
        // Reset error message.
        self.message = None;
        if self.content.pop().is_some() {
            self.altered = true;
            true
        } else {
            false
        }
    }

    // Remove last word. Return true if a word was removed.
    pub fn strip_word(&mut self) -> bool {
        // Reset error message.
        self.message = None;
        let iter = UnicodeSegmentation::split_word_bound_indices(
            self.content.as_str().trim_end());

        if let Some((index, _)) = iter.last() {
            self.content.truncate(index);
            self.altered = true;
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    // Clear input.
    pub fn clear(&mut self) {
        self.altered = true;
        self.content.clear();
    }

    // Clear input and message.
    pub fn reset(&mut self) {
        self.message = None;
        self.clear();
    }

    // Draw input buffer.
    pub fn draw<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        layout: &mut Layout,
    ) -> Result<(), std::io::Error>
    {
        // Write error message if present and return.
        if let Some(msg) = self.message {
            write!(stdout,
                "{}{}{}{}{}{}",
                cursor::Hide,
                cursor::Goto(
                    layout.buffer.col + (PROMPT.len() as u16),
                    layout.buffer.line),
                clear::UntilNewline,
                color::Fg(color::LightRed),
                &msg,
                color::Fg(color::Reset))?;
            return Ok(());
        }

        if !self.content.is_empty() {
            write!(stdout,
                "{}{}{}{}{}",
                cursor::Goto(layout.buffer.col, layout.buffer.line),
                clear::UntilNewline,
                PROMPT,
                cursor::Show,
                &self.content)?;
        } else {
            // Clear buffer display.
            write!(stdout,
                "{}{}{}",
                cursor::Goto(layout.buffer.col, layout.buffer.line),
                clear::CurrentLine,
                cursor::Hide)?;
        }
        Ok(())
    }

    // Draw error message at input buffer position.
    pub fn message(
        &mut self,
        msg: &'static str,
    ) {
        self.message = Some(msg);
        self.altered = true;
    }
}
