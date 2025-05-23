//! Interactive handling for download timeouts and user intervention

use anyhow::Result;
use std::io::{self, Write};

/// User choices when download times out
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutAction {
    /// Continue waiting with extended timeout
    Continue,
    /// Retry the download from the beginning
    Retry,
    /// Cancel the download
    Cancel,
}

/// Interactive timeout handler
pub struct InteractiveHandler {
    enabled: bool,
}

impl InteractiveHandler {
    /// Create a new interactive handler
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Handle timeout situation and get user choice
    pub async fn handle_timeout(
        &self,
        url: &str,
        timeout_secs: u64,
        diagnostic_report: &str,
    ) -> Result<TimeoutAction> {
        if !self.enabled {
            // Non-interactive mode - just return cancel
            return Ok(TimeoutAction::Cancel);
        }

        // Check if we're in a terminal environment
        if !atty::is(atty::Stream::Stdin) || !atty::is(atty::Stream::Stdout) {
            // Not in an interactive terminal
            return Ok(TimeoutAction::Cancel);
        }

        println!("\n{}", "=".repeat(80));
        println!("⚠️  Download Timeout - User Intervention Required");
        println!("{}", "=".repeat(80));
        println!();
        println!("Download URL: {}", url);
        println!("Timeout after: {} seconds", timeout_secs);
        println!();
        println!("Network Diagnostics Report:");
        println!("{}", diagnostic_report);
        println!();
        println!("What would you like to do?");
        println!();
        println!("1. Continue waiting (extend timeout by 5 minutes)");
        println!("2. Retry download from the beginning");
        println!("3. Cancel download");
        println!();

        loop {
            print!("Please choose an option (1-3): ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let choice = input.trim();
                    match choice {
                        "1" | "c" | "continue" => {
                            println!("✓ Continuing with extended timeout...");
                            return Ok(TimeoutAction::Continue);
                        }
                        "2" | "r" | "retry" => {
                            println!("✓ Retrying download...");
                            return Ok(TimeoutAction::Retry);
                        }
                        "3" | "q" | "cancel" | "quit" => {
                            println!("✓ Cancelling download...");
                            return Ok(TimeoutAction::Cancel);
                        }
                        _ => {
                            println!("❌ Invalid choice. Please enter 1, 2, or 3.");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error reading input: {}", e);
                    return Ok(TimeoutAction::Cancel);
                }
            }
        }
    }

    /// Show a simple progress message for non-interactive mode
    pub fn show_timeout_message(
        &self,
        url: &str,
        timeout_secs: u64,
        diagnostic_report: &str,
    ) {
        if self.enabled {
            return; // Interactive mode handles this differently
        }

        println!("\n{}", "=".repeat(80));
        println!("⚠️  Download Timeout");
        println!("{}", "=".repeat(80));
        println!();
        println!("Download URL: {}", url);
        println!("Timeout after: {} seconds", timeout_secs);
        println!();
        println!("Network Diagnostics Report:");
        println!("{}", diagnostic_report);
        println!();
        println!("💡 Tip: Use --interactive flag for timeout handling options");
        println!("💡 Tip: Use --timeout <seconds> to increase timeout duration");
        println!();
    }
}

/// Check if the current environment supports interactive input
pub fn supports_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}

/// Get user confirmation for a yes/no question
pub async fn get_user_confirmation(prompt: &str) -> Result<bool> {
    if !supports_interactive() {
        return Ok(false);
    }

    loop {
        print!("{} (y/n): ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let choice = input.trim().to_lowercase();
                match choice.as_str() {
                    "y" | "yes" => return Ok(true),
                    "n" | "no" => return Ok(false),
                    _ => {
                        println!("Please enter 'y' for yes or 'n' for no.");
                        continue;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                return Ok(false);
            }
        }
    }
}
