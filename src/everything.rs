use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::card::{parse_hand_any, Card};
use crate::sim::run_simulation;

/// Bump this when the output schema gains new fields.
/// Files with a lower version will be re-computed on the next run.
pub const CURRENT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Canonical hand types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum HandType {
    Pair(u8),         // e.g. AA  → rank = 14
    Suited(u8, u8),   // e.g. AKs → rank1=14, rank2=13  (rank1 > rank2)
    Offsuit(u8, u8),  // e.g. KQo → rank1=13, rank2=12  (rank1 > rank2)
}

impl HandType {
    fn name(&self) -> String {
        match self {
            HandType::Pair(r)         => format!("{0}{0}", rank_char(*r)),
            HandType::Suited(r1, r2)  => format!("{}{}s", rank_char(*r1), rank_char(*r2)),
            HandType::Offsuit(r1, r2) => format!("{}{}o", rank_char(*r1), rank_char(*r2)),
        }
    }

    fn resolve(&self, used: &HashSet<Card>) -> Result<[Card; 2], String> {
        parse_hand_any(&self.name(), used)
    }
}

fn rank_char(r: u8) -> char {
    match r {
        2..=9 => (b'0' + r) as char,
        10 => 'T',
        11 => 'J',
        12 => 'Q',
        13 => 'K',
        14 => 'A',
        _  => '?',
    }
}

/// All 169 canonical preflop hand types, ordered:
/// pairs (AA→22), suited (AKs→32s), offsuit (AKo→32o).
fn all_hand_types() -> Vec<HandType> {
    let mut types = Vec::with_capacity(169);
    for r in (2..=14u8).rev() {
        types.push(HandType::Pair(r));
    }
    for r1 in (2..=14u8).rev() {
        for r2 in (2..r1).rev() {
            types.push(HandType::Suited(r1, r2));
        }
    }
    for r1 in (2..=14u8).rev() {
        for r2 in (2..r1).rev() {
            types.push(HandType::Offsuit(r1, r2));
        }
    }
    debug_assert_eq!(types.len(), 169);
    types
}

// ---------------------------------------------------------------------------
// JSON schema
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct MatchupOutput {
    /// Schema version — bump CURRENT_VERSION to trigger re-computation.
    version: u32,
    /// Canonical shorthand for hand 1 (e.g. "AKs", "QQ").
    hand1: String,
    /// Canonical shorthand for hand 2.
    hand2: String,
    /// The specific cards chosen for hand 1 (e.g. "AhKh").
    hand1_resolved: String,
    /// The specific cards chosen for hand 2.
    hand2_resolved: String,
    wins1: u64,
    wins2: u64,
    ties: u64,
    total: u64,
    win_pct1: f64,
    win_pct2: f64,
    tie_pct: f64,
}

// ---------------------------------------------------------------------------
// Resumability helpers
// ---------------------------------------------------------------------------

/// Returns true if the file exists, parses as valid JSON, and has
/// `version >= required_version`.
fn is_complete(path: &Path, required_version: u32) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    val.get("version")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32 >= required_version)
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Main driver
// ---------------------------------------------------------------------------

pub fn run_everything(output_dir: &str) {
    let dir = Path::new(output_dir);
    fs::create_dir_all(dir).expect("failed to create output directory");

    let types = all_hand_types();
    let n = types.len(); // 169
    let total_matchups = n * (n + 1) / 2; // 14,365

    // Count how many are already done so we can give accurate progress
    let already_done: usize = (0..n)
        .flat_map(|i| (i..n).map(move |j| (i, j)))
        .filter(|(i, j)| {
            let name = format!("{}_vs_{}.json", types[*i].name(), types[*j].name());
            is_complete(&dir.join(name), CURRENT_VERSION)
        })
        .count();

    if already_done == total_matchups {
        println!("All {} matchups already complete (version {}).", total_matchups, CURRENT_VERSION);
        return;
    }

    let remaining = total_matchups - already_done;
    println!(
        "Running {} matchups ({} remaining, {} already complete, version {}).",
        total_matchups, remaining, already_done, CURRENT_VERSION
    );
    println!("Output directory: {}\n", output_dir);

    let start = Instant::now();
    let mut computed = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for i in 0..n {
        for j in i..n {
            let h1 = &types[i];
            let h2 = &types[j];

            let filename = format!("{}_vs_{}.json", h1.name(), h2.name());
            let path = dir.join(&filename);

            if is_complete(&path, CURRENT_VERSION) {
                skipped += 1;
                continue;
            }

            // Resolve shorthand to specific cards, avoiding conflicts
            let mut used = HashSet::new();
            let cards1 = match h1.resolve(&used) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  error: {} vs {} — {}", h1.name(), h2.name(), e);
                    errors += 1;
                    continue;
                }
            };
            used.extend(cards1.iter());
            let cards2 = match h2.resolve(&used) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  error: {} vs {} — {}", h1.name(), h2.name(), e);
                    errors += 1;
                    continue;
                }
            };

            let result = run_simulation(cards1, cards2);

            let output = MatchupOutput {
                version: CURRENT_VERSION,
                hand1: h1.name(),
                hand2: h2.name(),
                hand1_resolved: format!(
                    "{}{}",
                    cards1[0].to_string(),
                    cards1[1].to_string()
                ),
                hand2_resolved: format!(
                    "{}{}",
                    cards2[0].to_string(),
                    cards2[1].to_string()
                ),
                wins1: result.wins1,
                wins2: result.wins2,
                ties: result.ties,
                total: result.total,
                win_pct1: (result.win_pct1() * 100.0).round() / 100.0,
                win_pct2: (result.win_pct2() * 100.0).round() / 100.0,
                tie_pct: (result.tie_pct() * 100.0).round() / 100.0,
            };

            let json =
                serde_json::to_string_pretty(&output).expect("serialization failed");
            // Write atomically via a temp file to avoid corrupt output on interrupt
            let tmp_path = path.with_extension("tmp");
            fs::write(&tmp_path, json).expect("failed to write temp file");
            fs::rename(&tmp_path, &path).expect("failed to rename temp file");

            computed += 1;

            // Progress: print every 50 computed or on the last one
            let total_done = computed + skipped;
            if computed % 50 == 0 || total_done == total_matchups {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = computed as f64 / elapsed.max(0.001);
                let eta_secs = (remaining.saturating_sub(computed)) as f64 / rate;
                println!(
                    "  [{}/{}] {:.1}s elapsed, ~{:.0}s remaining  (last: {} vs {} → {:.2}% | {:.2}% | {:.2}%)",
                    total_done,
                    total_matchups,
                    elapsed,
                    eta_secs,
                    h1.name(),
                    h2.name(),
                    result.win_pct1(),
                    result.win_pct2(),
                    result.tie_pct(),
                );
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!("\nFinished in {:.1}s — {} computed, {} skipped, {} errors.",
        elapsed, computed, skipped, errors);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_hand_types_count() {
        assert_eq!(all_hand_types().len(), 169);
    }

    #[test]
    fn test_hand_type_names() {
        assert_eq!(HandType::Pair(14).name(), "AA");
        assert_eq!(HandType::Pair(2).name(), "22");
        assert_eq!(HandType::Suited(14, 13).name(), "AKs");
        assert_eq!(HandType::Suited(3, 2).name(), "32s");
        assert_eq!(HandType::Offsuit(13, 12).name(), "KQo");
        assert_eq!(HandType::Offsuit(3, 2).name(), "32o");
    }

    #[test]
    fn test_total_matchups() {
        let n = 169usize;
        assert_eq!(n * (n + 1) / 2, 14_365);
    }

    #[test]
    fn test_all_hand_types_resolvable() {
        // Every hand type should resolve successfully against an empty used set
        for ht in all_hand_types() {
            let result = ht.resolve(&HashSet::new());
            assert!(result.is_ok(), "{} failed to resolve: {:?}", ht.name(), result);
        }
    }

    #[test]
    fn test_self_matchup_resolvable() {
        // e.g. AA vs AA should use different suits for each hand
        for ht in all_hand_types() {
            let mut used = HashSet::new();
            let h1 = ht.resolve(&used).expect("hand1 resolve failed");
            used.extend(h1.iter());
            let h2 = ht.resolve(&used).expect("hand2 resolve failed");
            // All 4 cards must be distinct
            let all = [h1[0], h1[1], h2[0], h2[1]];
            let unique: HashSet<_> = all.iter().collect();
            assert_eq!(unique.len(), 4, "collision in {} vs {}", ht.name(), ht.name());
        }
    }

    #[test]
    fn test_all_matchup_pairs_no_card_collision() {
        // For every (i, j) pair, both hands must resolve to 4 distinct cards.
        // Checks the full 14,365-pair matrix without running any simulations.
        let types = all_hand_types();
        let n = types.len();
        for i in 0..n {
            for j in i..n {
                let mut used = HashSet::new();
                let h1 = types[i]
                    .resolve(&used)
                    .unwrap_or_else(|e| panic!("i={} ({}) failed: {}", i, types[i].name(), e));
                used.extend(h1.iter());
                let h2 = types[j]
                    .resolve(&used)
                    .unwrap_or_else(|e| panic!("j={} ({}) failed: {}", j, types[j].name(), e));
                let all = [h1[0], h1[1], h2[0], h2[1]];
                let unique: HashSet<_> = all.iter().collect();
                assert_eq!(
                    unique.len(), 4,
                    "card collision: {} vs {}",
                    types[i].name(), types[j].name()
                );
            }
        }
    }

    // Spot-check a handful of known equities against the full simulation.
    // Kept to a small sample so this test finishes in a few seconds.
    #[test]
    fn test_equity_spot_checks() {
        let cases: &[(&str, &str, f64, f64)] = &[
            // (hand1, hand2, expected_win_pct1, tolerance)
            ("AA", "KK",  82.0, 1.5),   // AA ~82%
            ("AA", "22",  80.0, 2.0),   // AA dominates
            ("KK", "QQ",  81.0, 2.0),   // KK dominates
            ("AKs", "QQ", 46.0, 2.0),   // QQ slight favourite
            ("AKo", "QQ", 43.0, 2.0),   // QQ more comfortably ahead
            ("72o", "AKs",  25.0, 4.0), // 72o is a big dog
        ];

        for &(h1_spec, h2_spec, expected_pct1, tol) in cases {
            let mut used = HashSet::new();
            let h1 = parse_hand_any(h1_spec, &used).expect("parse h1");
            used.extend(h1.iter());
            let h2 = parse_hand_any(h2_spec, &used).expect("parse h2");

            let result = run_simulation(h1, h2);
            let pct1 = result.win_pct1();
            assert!(
                (pct1 - expected_pct1).abs() <= tol,
                "{} vs {}: win_pct1={:.2}%, expected ~{}% ±{}",
                h1_spec, h2_spec, pct1, expected_pct1, tol
            );
        }
    }

    #[test]
    fn test_json_round_trip() {
        // Build a MatchupOutput, serialize, deserialize, verify fields survive.
        let output = MatchupOutput {
            version: CURRENT_VERSION,
            hand1: "AA".to_string(),
            hand2: "KK".to_string(),
            hand1_resolved: "AhAd".to_string(),
            hand2_resolved: "KhKd".to_string(),
            wins1: 1_410_336,
            wins2: 292_660,
            ties: 9_308,
            total: 1_712_304,
            win_pct1: 82.36,
            win_pct2: 17.09,
            tie_pct: 0.54,
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: MatchupOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, CURRENT_VERSION);
        assert_eq!(parsed.hand1, "AA");
        assert_eq!(parsed.wins1, 1_410_336);
        assert_eq!(parsed.total, 1_712_304);
    }

    #[test]
    fn test_is_complete_missing_file() {
        assert!(!is_complete(Path::new("/nonexistent/path/foo.json"), 1));
    }

    #[test]
    fn test_is_complete_version_check() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"version": 1, "other": "data"}}"#).unwrap();
        assert!(is_complete(tmp.path(), 1));   // exact version → complete
        assert!(!is_complete(tmp.path(), 2));  // requires newer version → stale
    }
}
