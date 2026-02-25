use crate::card::{full_deck, Card};
use crate::eval::eval_7;

pub struct SimResult {
    pub wins1: u64,
    pub wins2: u64,
    pub ties: u64,
    pub total: u64,
}

impl SimResult {
    pub fn win_pct1(&self) -> f64 {
        self.wins1 as f64 / self.total as f64 * 100.0
    }
    pub fn win_pct2(&self) -> f64 {
        self.wins2 as f64 / self.total as f64 * 100.0
    }
    pub fn tie_pct(&self) -> f64 {
        self.ties as f64 / self.total as f64 * 100.0
    }
}

/// Exhaustive enumeration of all C(48,5) = 1,712,304 possible boards.
pub fn run_simulation(hand1: [Card; 2], hand2: [Card; 2]) -> SimResult {
    // Build deck minus the 4 hole cards
    let hole_cards = [hand1[0], hand1[1], hand2[0], hand2[1]];
    let deck: Vec<Card> = full_deck()
        .into_iter()
        .filter(|c| !hole_cards.contains(c))
        .collect();

    let n = deck.len(); // should be 48
    let mut wins1 = 0u64;
    let mut wins2 = 0u64;
    let mut ties = 0u64;
    let mut total = 0u64;

    for i0 in 0..n - 4 {
        for i1 in i0 + 1..n - 3 {
            for i2 in i1 + 1..n - 2 {
                for i3 in i2 + 1..n - 1 {
                    for i4 in i3 + 1..n {
                        let community = [deck[i0], deck[i1], deck[i2], deck[i3], deck[i4]];

                        let seven1 = [
                            hand1[0], hand1[1],
                            community[0], community[1], community[2],
                            community[3], community[4],
                        ];
                        let seven2 = [
                            hand2[0], hand2[1],
                            community[0], community[1], community[2],
                            community[3], community[4],
                        ];

                        let score1 = eval_7(&seven1);
                        let score2 = eval_7(&seven2);

                        match score1.cmp(&score2) {
                            std::cmp::Ordering::Greater => wins1 += 1,
                            std::cmp::Ordering::Less => wins2 += 1,
                            std::cmp::Ordering::Equal => ties += 1,
                        }
                        total += 1;
                    }
                }
            }
        }
    }

    SimResult { wins1, wins2, ties, total }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;

    fn c(s: &str) -> Card {
        Card::parse(s).unwrap()
    }

    #[test]
    fn test_total_boards() {
        // Quick sanity check: AA vs KK should enumerate exactly 1,712,304 boards
        let hand1 = [c("Ah"), c("Ad")];
        let hand2 = [c("Kh"), c("Kd")];
        let result = run_simulation(hand1, hand2);
        assert_eq!(result.total, 1_712_304);
        assert_eq!(result.wins1 + result.wins2 + result.ties, result.total);
    }

    #[test]
    fn test_aa_vs_kk_equity() {
        // AA vs KK: AA should win approximately 81-82%
        let hand1 = [c("Ah"), c("Ad")];
        let hand2 = [c("Kh"), c("Kd")];
        let result = run_simulation(hand1, hand2);
        let pct1 = result.win_pct1();
        // Exact equity: AA wins ~81.9%
        assert!(pct1 > 80.0 && pct1 < 84.0, "AA win% was {:.2}", pct1);
    }

    #[test]
    fn test_mirror_hands_roughly_equal() {
        // AhKh vs AcKc: nearly identical equity (same ranks, different suits but both suited)
        let hand1 = [c("Ah"), c("Kh")];
        let hand2 = [c("Ac"), c("Kc")];
        let result = run_simulation(hand1, hand2);
        // Should be very close to 50/50 with slight edge to neither
        let pct1 = result.win_pct1();
        let pct2 = result.win_pct2();
        assert!((pct1 - pct2).abs() < 1.0, "pct1={:.2} pct2={:.2}", pct1, pct2);
    }
}
