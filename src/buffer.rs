use std::io::Write;
use termion::{clear, cursor, color};
use termion::raw::RawTerminal;
use crate::layout::Layout;
use crate::utils;

const PROMPT: &str = "Add alarm: ";


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

    pub fn read(&mut self) -> &String {
        self.altered = false;
        &self.content
    }

    pub fn push(&mut self, value: char) {
        self.altered = true;
        self.message = None;
        self.content.push(value);
    }

    pub fn pop(&mut self) -> Option<char> {
        self.altered = true;
        self.message = None;
        self.content.pop()
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
            layout.cursor.col =
                layout.buffer.col
                + 11
                + utils::unicode_length(&self.content);
            // TODO: This would be a much better alternative, but panics because
            // of interference with async_reader.
            //layout.cursor.col = stdout.cursor_pos()?.0;
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
