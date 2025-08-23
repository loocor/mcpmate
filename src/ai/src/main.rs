//! MCPMate AI module: Text MCP configuration extractor
//!
//! Based on Qwen2.5 0.5B model for local inference, converts input text to MCP service configuration JSON

use clap::Parser;
use mcpmate_ai::{
    Result, TextMcpExtractor,
    config::{Args, ExtractorConfig},
    utils::{InputReader, set_debug_mode},
};

fn main() -> Result<()> {
    let args = Args::parse();

    // Set debug mode
    set_debug_mode(args.debug);

    // Get input text
    let input_text = InputReader::get_input_text(args.text.as_ref(), args.file.as_ref(), args.stdin)?;

    // Create extractor configuration
    let config = ExtractorConfig::from_args(&args);

    // Create and initialize extractor
    let mut extractor = TextMcpExtractor::new(config)?;

    // Execute extraction
    match extractor.extract(&input_text) {
        Ok(config) => {
            println!("\n📋 Generated MCP Configuration:");
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        Err(e) => {
            eprintln!("❌ Extraction failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
