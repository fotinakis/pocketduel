use std::collections::HashSet;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Rank(pub u8);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Suit(pub u8);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

fn parse_rank_byte(b: u8) -> Result<u8, String> {
    match b.to_ascii_uppercase() {
        b'2' => Ok(2),
        b'3' => Ok(3),
        b'4' => Ok(4),
        b'5' => Ok(5),
        b'6' => Ok(6),
        b'7' => Ok(7),
        b'8' => Ok(8),
        b'9' => Ok(9),
        b'T' => Ok(10),
        b'J' => Ok(11),
        b'Q' => Ok(12),
        b'K' => Ok(13),
        b'A' => Ok(14),
        c => Err(format!("unknown rank '{}'", c as char)),
    }
}

fn rank_to_char(r: u8) -> char {
    match r {
        2..=9 => (b'0' + r) as char,
        10 => 'T',
        11 => 'J',
        12 => 'Q',
        13 => 'K',
        14 => 'A',
        _ => '?',
    }
}

impl Card {
    pub fn parse(s: &str) -> Result<Card, String> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return Err(format!("card must be 2 characters, got: {}", s));
        }
        let rank_val = parse_rank_byte(bytes[0])?;
        let suit_val = match bytes[1].to_ascii_lowercase() {
            b'h' => 0,
            b'd' => 1,
            b'c' => 2,
            b's' => 3,
            c => return Err(format!("unknown suit '{}'", c as char)),
        };
        Ok(Card {
            rank: Rank(rank_val),
            suit: Suit(suit_val),
        })
    }

    pub fn to_string(&self) -> String {
        format!("{}{}", rank_to_char(self.rank.0), match self.suit.0 {
            0 => 'h',
            1 => 'd',
            2 => 'c',
            3 => 's',
            _ => '?',
        })
    }
}

/// Parse a concrete hand: "AhKh" (4-char) or "Ah Kh" (space-separated).
pub fn parse_hand(s: &str) -> Result<[Card; 2], String> {
    let parts: Vec<&str> = if s.contains(' ') {
        s.split_whitespace().collect()
    } else if s.len() == 4 {
        vec![&s[0..2], &s[2..4]]
    } else {
        return Err(format!("cannot parse hand: {}", s));
    };
    if parts.len() != 2 {
        return Err(format!("hand must have exactly 2 cards, got: {}", s));
    }
    let c1 = Card::parse(parts[0])?;
    let c2 = Card::parse(parts[1])?;
    Ok([c1, c2])
}

// --- Shorthand suit assignment helpers ---

/// Pick two different suits for a paired rank (e.g. AA → AhAd).
fn assign_pair(rank: u8, used: &HashSet<Card>) -> Result<[Card; 2], String> {
    let mut found = [Card::default(); 2];
    let mut count = 0;
    for suit in 0..4u8 {
        let c = Card { rank: Rank(rank), suit: Suit(suit) };
        if !used.contains(&c) {
            found[count] = c;
            count += 1;
            if count == 2 {
                break;
            }
        }
    }
    if count < 2 {
        Err(format!(
            "cannot assign two suits for {}{}; too many cards already in use",
            rank_to_char(rank),
            rank_to_char(rank)
        ))
    } else {
        Ok(found)
    }
}

/// Pick a suit where both ranks are available (e.g. AKs → AhKh).
fn assign_suited(r1: u8, r2: u8, used: &HashSet<Card>) -> Result<[Card; 2], String> {
    for suit in 0..4u8 {
        let c1 = Card { rank: Rank(r1), suit: Suit(suit) };
        let c2 = Card { rank: Rank(r2), suit: Suit(suit) };
        if !used.contains(&c1) && !used.contains(&c2) {
            return Ok([c1, c2]);
        }
    }
    Err(format!(
        "cannot assign a suit for {}{}s; all suited combinations already in use",
        rank_to_char(r1),
        rank_to_char(r2)
    ))
}

/// Pick different suits for two ranks (e.g. AKo → AhKd).
fn assign_offsuit(r1: u8, r2: u8, used: &HashSet<Card>) -> Result<[Card; 2], String> {
    let suit1 = (0..4u8)
        .find(|&s| !used.contains(&Card { rank: Rank(r1), suit: Suit(s) }));
    let Some(s1) = suit1 else {
        return Err(format!(
            "no available suit for {} in {}{}o",
            rank_to_char(r1), rank_to_char(r1), rank_to_char(r2)
        ));
    };
    // r2 must get a suit different from s1 (to be genuinely offsuit)
    let suit2 = (0..4u8)
        .find(|&s| s != s1 && !used.contains(&Card { rank: Rank(r2), suit: Suit(s) }));
    let Some(s2) = suit2 else {
        return Err(format!(
            "no available offsuit assignment for {}{}o",
            rank_to_char(r1), rank_to_char(r2)
        ));
    };
    Ok([
        Card { rank: Rank(r1), suit: Suit(s1) },
        Card { rank: Rank(r2), suit: Suit(s2) },
    ])
}

/// Parse a hand in any supported format, avoiding cards already in `used`.
///
/// Supported formats:
/// - Concrete: `"AhKh"` or `"Ah Kh"`
/// - Paired shorthand: `"AA"`, `"KK"`, `"22"`, etc.
/// - Suited shorthand: `"AKs"`, `"T9s"`, etc.
/// - Offsuit shorthand: `"AKo"`, `"KQo"`, etc.
///
/// For shorthand, suits are chosen automatically using the lowest available
/// suits that don't conflict with `used`.
pub fn parse_hand_any(s: &str, used: &HashSet<Card>) -> Result<[Card; 2], String> {
    // Try concrete format first
    if s.contains(' ') || s.len() == 4 {
        let hand = parse_hand(s)?;
        for c in hand.iter() {
            if used.contains(c) {
                return Err(format!("card {} conflicts with the other hand", c.to_string()));
            }
        }
        if hand[0] == hand[1] {
            return Err("hand contains duplicate cards".to_string());
        }
        return Ok(hand);
    }

    let b = s.as_bytes();
    let hand = match b.len() {
        2 => {
            let r1 = parse_rank_byte(b[0])
                .map_err(|e| format!("cannot parse '{}': {}", s, e))?;
            let r2 = parse_rank_byte(b[1])
                .map_err(|e| format!("cannot parse '{}': {}", s, e))?;
            if r1 != r2 {
                return Err(format!(
                    "cannot parse '{}': 2-character hands must be a pair (same rank, e.g. AA)",
                    s
                ));
            }
            assign_pair(r1, used)?
        }
        3 => {
            let r1 = parse_rank_byte(b[0])
                .map_err(|e| format!("cannot parse '{}': {}", s, e))?;
            let r2 = parse_rank_byte(b[1])
                .map_err(|e| format!("cannot parse '{}': {}", s, e))?;
            match b[2].to_ascii_lowercase() {
                b's' => assign_suited(r1, r2, used)?,
                b'o' => assign_offsuit(r1, r2, used)?,
                c => {
                    return Err(format!(
                        "cannot parse '{}': suffix '{}' not recognized (use 's' for suited or 'o' for offsuit)",
                        s, c as char
                    ))
                }
            }
        }
        _ => {
            return Err(format!(
                "cannot parse '{}': expected e.g. AhKh, AKs, KQo, or AA",
                s
            ))
        }
    };

    Ok(hand)
}

pub fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for suit in 0..4u8 {
        for rank in 2..=14u8 {
            deck.push(Card {
                rank: Rank(rank),
                suit: Suit(suit),
            });
        }
    }
    deck
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> HashSet<Card> {
        HashSet::new()
    }

    #[test]
    fn test_parse_card() {
        let c = Card::parse("Ah").unwrap();
        assert_eq!(c.rank.0, 14);
        assert_eq!(c.suit.0, 0);

        let c = Card::parse("Ts").unwrap();
        assert_eq!(c.rank.0, 10);
        assert_eq!(c.suit.0, 3);

        let c = Card::parse("2c").unwrap();
        assert_eq!(c.rank.0, 2);
        assert_eq!(c.suit.0, 2);
    }

    #[test]
    fn test_parse_card_case_insensitive() {
        let c = Card::parse("ah").unwrap();
        assert_eq!(c.rank.0, 14);
        let c = Card::parse("tS").unwrap();
        assert_eq!(c.rank.0, 10);
        assert_eq!(c.suit.0, 3);
    }

    #[test]
    fn test_parse_hand_no_space() {
        let h = parse_hand("AhKh").unwrap();
        assert_eq!(h[0].rank.0, 14);
        assert_eq!(h[1].rank.0, 13);
    }

    #[test]
    fn test_parse_hand_with_space() {
        let h = parse_hand("Ah Kh").unwrap();
        assert_eq!(h[0].rank.0, 14);
        assert_eq!(h[1].rank.0, 13);
    }

    #[test]
    fn test_full_deck() {
        let deck = full_deck();
        assert_eq!(deck.len(), 52);
        let set: std::collections::HashSet<Card> = deck.iter().cloned().collect();
        assert_eq!(set.len(), 52);
    }

    // --- parse_hand_any tests ---

    #[test]
    fn test_shorthand_pair() {
        let h = parse_hand_any("AA", &empty()).unwrap();
        assert_eq!(h[0].rank.0, 14);
        assert_eq!(h[1].rank.0, 14);
        assert_ne!(h[0].suit, h[1].suit);
    }

    #[test]
    fn test_shorthand_suited() {
        let h = parse_hand_any("AKs", &empty()).unwrap();
        assert_eq!(h[0].rank.0, 14);
        assert_eq!(h[1].rank.0, 13);
        assert_eq!(h[0].suit, h[1].suit);
    }

    #[test]
    fn test_shorthand_offsuit() {
        let h = parse_hand_any("KQo", &empty()).unwrap();
        assert_eq!(h[0].rank.0, 13);
        assert_eq!(h[1].rank.0, 12);
        assert_ne!(h[0].suit, h[1].suit);
    }

    #[test]
    fn test_shorthand_case_insensitive() {
        let h = parse_hand_any("aks", &empty()).unwrap();
        assert_eq!(h[0].rank.0, 14);
        assert_eq!(h[1].rank.0, 13);
        assert_eq!(h[0].suit, h[1].suit);
    }

    #[test]
    fn test_shorthand_conflict_avoidance() {
        // AKs takes AhKh; AKo should avoid those and pick a different offsuit combo
        let mut used = HashSet::new();
        let h1 = parse_hand_any("AKs", &used).unwrap();
        for c in h1.iter() { used.insert(*c); }
        let h2 = parse_hand_any("AKo", &used).unwrap();
        // h2 must not overlap h1
        for c in h2.iter() {
            assert!(!used.contains(c), "conflict: {:?}", c);
        }
        // h2 must be offsuit
        assert_ne!(h2[0].suit, h2[1].suit);
    }

    #[test]
    fn test_shorthand_two_suited_same_ranks() {
        // AKs vs AKs — should pick different suits
        let mut used = HashSet::new();
        let h1 = parse_hand_any("AKs", &used).unwrap();
        for c in h1.iter() { used.insert(*c); }
        let h2 = parse_hand_any("AKs", &used).unwrap();
        assert_ne!(h1[0], h2[0]);
        assert_eq!(h2[0].suit, h2[1].suit);
    }

    #[test]
    fn test_concrete_conflict_detected() {
        let mut used = HashSet::new();
        let h1 = parse_hand_any("AhKh", &used).unwrap();
        for c in h1.iter() { used.insert(*c); }
        assert!(parse_hand_any("AhQd", &used).is_err());
    }

    #[test]
    fn test_shorthand_two_char_non_pair_error() {
        assert!(parse_hand_any("AK", &empty()).is_err());
    }
}
