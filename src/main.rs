use boxpacker::cli::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    match boxpacker::app::run(&cli) {
        Ok(summary) => {
            println!(
                "Packed {} of {} items (status: {}); wrote {} and {}",
                summary.solution().placed_item_count(),
                summary.solution().placed_item_count() + summary.solution().unplaced_item_count(),
                summary.optimality(),
                summary.json_output_path().display(),
                summary.html_output_path().display(),
            );
        }
        Err(error) => {
            eprintln!("boxpacker: {error}");
            std::process::exit(1);
        }
    }
}
