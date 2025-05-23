//! Progress display utilities for runtime downloads

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::runtime::types::DownloadProgress;

/// Single-line progress bar that updates in place
pub struct InlineProgressBar {
    last_update: Instant,
    update_interval: Duration,
    terminal_width: usize,
}

impl Default for InlineProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineProgressBar {
    /// Create a new inline progress bar
    pub fn new() -> Self {
        let terminal_width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);

        Self {
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100), // update frequency: every 100ms
            terminal_width,
        }
    }

    /// Update the progress bar display
    pub fn update(
        &mut self,
        progress: &DownloadProgress,
    ) {
        // limit update frequency to avoid flickering
        if self.last_update.elapsed() < self.update_interval {
            return;
        }
        self.last_update = Instant::now();

        let line = self.format_progress_line(progress);
        self.print_line(&line);
    }

    /// Format a single progress line
    fn format_progress_line(
        &self,
        progress: &DownloadProgress,
    ) -> String {
        let stage_text = format!("{}", progress.stage);

        if let (Some(percentage), Some(total)) = (progress.percentage(), progress.total) {
            // with specific progress
            let bar_width = 20;
            let filled = (percentage * bar_width as f64 / 100.0) as usize;
            let empty = bar_width - filled;

            let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            let speed_text = if let Some(speed) = progress.speed {
                format!(" {}/s", format_bytes(speed))
            } else {
                String::new()
            };

            let size_text = format!(
                " {}/{}",
                format_bytes(progress.downloaded),
                format_bytes(total)
            );

            format!(
                "{} {} {:.1}%{}{}",
                stage_text, progress_bar, percentage, size_text, speed_text
            )
        } else {
            // no specific progress (e.g. extracting)
            let spinner = self.get_spinner();
            let message = progress.message.as_deref().unwrap_or("");

            format!("{} {} {}", stage_text, spinner, message)
        }
    }

    /// Get a spinning animation character
    fn get_spinner(&self) -> &'static str {
        let chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let index = (self.last_update.elapsed().as_millis() / 100) % chars.len() as u128;
        chars[index as usize]
    }

    /// Print a line with carriage return (overwrites current line)
    fn print_line(
        &self,
        line: &str,
    ) {
        // clear current line and print new content
        print!("\r{:<width$}", line, width = self.terminal_width);
        io::stdout().flush().unwrap();
    }

    /// Finish the progress bar (move to next line)
    pub fn finish(
        &self,
        final_message: &str,
    ) {
        println!("\r{:<width$}", final_message, width = self.terminal_width);
    }

    /// Clear the current line
    pub fn clear(&self) {
        print!("\r{:<width$}\r", "", width = self.terminal_width);
        io::stdout().flush().unwrap();
    }
}

/// Format bytes in human readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Multi-line progress display (fallback for non-terminal environments)
pub struct MultiLineProgress;

impl MultiLineProgress {
    pub fn update(progress: &DownloadProgress) {
        if let Some(percentage) = progress.percentage() {
            println!(
                "[{}] {:.1}% - {}",
                progress.stage,
                percentage,
                progress.message.as_deref().unwrap_or("")
            );
        } else {
            println!(
                "[{}] {}",
                progress.stage,
                progress.message.as_deref().unwrap_or("")
            );
        }
    }
}

/// Detect if we're in a terminal that supports inline updates
pub fn supports_inline_progress() -> bool {
    atty::is(atty::Stream::Stdout)
}
