use std::collections::HashSet;

use clap::Parser;

mod card;
mod eval;
mod everything;
mod sim;

use card::parse_hand_any;
use sim::run_simulation;

#[derive(Parser)]
#[command(name = "pocketduel", about = "Texas Hold'em preflop equity calculator")]
struct Cli {
    /// First hand, e.g. AhKh, AKs, KQo, or AA
    #[arg(long, conflicts_with = "everything")]
    hand1: Option<String>,

    /// Second hand, e.g. AsKs, KQo, or KK
    #[arg(long, conflicts_with = "everything")]
    hand2: Option<String>,

    /// Compute all 14,365 preflop matchups and write one JSON file each
    #[arg(long)]
    everything: bool,

    /// Output directory for --everything (default: ./equity_data)
    #[arg(long, default_value = "equity_data")]
    output_dir: String,
}

fn main() {
    let cli = Cli::parse();

    if cli.everything {
        everything::run_everything(&cli.output_dir);
        return;
    }

    // Single matchup mode — both --hand1 and --hand2 are required
    let h1_spec = cli.hand1.unwrap_or_else(|| {
        eprintln!("Error: --hand1 is required (or use --everything)");
        std::process::exit(1);
    });
    let h2_spec = cli.hand2.unwrap_or_else(|| {
        eprintln!("Error: --hand2 is required (or use --everything)");
        std::process::exit(1);
    });

    let mut used = HashSet::new();

    let hand1 = match parse_hand_any(&h1_spec, &used) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error parsing --hand1: {}", e);
            std::process::exit(1);
        }
    };
    used.extend(hand1.iter());

    let hand2 = match parse_hand_any(&h2_spec, &used) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error parsing --hand2: {}", e);
            std::process::exit(1);
        }
    };

    let h1_str = format!("{}{}", hand1[0].to_string(), hand1[1].to_string());
    let h2_str = format!("{}{}", hand2[0].to_string(), hand2[1].to_string());

    println!("Running exhaustive simulation (1,712,304 boards)...\n");

    let result = run_simulation(hand1, hand2);

    println!("Results:");
    println!(
        "  Hand 1 ({}) wins: {:>8} ({:.2}%)",
        h1_str, result.wins1, result.win_pct1()
    );
    println!(
        "  Hand 2 ({}) wins: {:>8} ({:.2}%)",
        h2_str, result.wins2, result.win_pct2()
    );
    println!(
        "  Ties:             {:>8}  ({:.2}%)",
        result.ties, result.tie_pct()
    );
    println!("  Total boards:    {:>8}", result.total);
}
