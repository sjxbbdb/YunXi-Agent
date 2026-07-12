use anyhow::{Context, Result};
use std::io::{BufRead, Write};

pub(crate) trait InteractiveInput {
    fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>>;
    fn read_response(&mut self, prompt: &str) -> Result<Option<String>>;
    fn print_eof_message(&self) -> bool;
}

pub(crate) struct PlainInput<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    prompt_enabled: bool,
}

impl<R: BufRead, W: Write> PlainInput<R, W> {
    pub(crate) fn new(reader: R, writer: W, prompt_enabled: bool) -> Self {
        Self {
            reader,
            writer,
            prompt_enabled,
        }
    }

    fn read_line(&mut self, prompt: &str) -> Result<Option<String>> {
        if self.prompt_enabled && !prompt.is_empty() {
            write!(self.writer, "{prompt}")?;
            self.writer.flush()?;
        }
        let mut input = String::new();
        let bytes = self
            .reader
            .read_line(&mut input)
            .context("failed to read interactive input")?;
        if bytes == 0 {
            return Ok(None);
        }
        Ok(Some(input.trim_end_matches(['\r', '\n']).to_string()))
    }
}

impl<R: BufRead, W: Write> InteractiveInput for PlainInput<R, W> {
    fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>> {
        self.read_line(prompt)
    }

    fn read_response(&mut self, prompt: &str) -> Result<Option<String>> {
        self.read_line(prompt)
    }

    fn print_eof_message(&self) -> bool {
        !self.prompt_enabled
    }
}
