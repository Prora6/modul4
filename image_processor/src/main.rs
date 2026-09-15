//! CLI entry point for the image processor.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use image_processor::process;

#[derive(Debug, Parser)]
#[command(
    name = "image_processor",
    about = "Load an image, apply a dynamic processing plugin, and save the result as PNG.",
    version
)]
struct Cli {
    /// Path to the source image (PNG recommended).
    #[arg(long)]
    input: PathBuf,

    /// Path where the processed PNG will be written.
    #[arg(long)]
    output: PathBuf,

    /// Bare plugin name without prefix or extension (e.g. `mirror`).
    #[arg(long)]
    plugin: String,

    /// Path to a text file with plugin parameters.
    #[arg(long)]
    params: PathBuf,

    /// Directory containing the plugin shared library.
    #[arg(long, default_value = "target/debug")]
    plugin_path: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match process(
        &cli.input,
        &cli.output,
        &cli.plugin,
        &cli.params,
        &cli.plugin_path,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
