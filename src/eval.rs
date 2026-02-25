use crate::card::Card;

/// Packed hand score: bits [47:40]=category, [39:32]=tb1, [31:24]=tb2,
/// [23:16]=tb3, [15:8]=tb4, [7:0]=tb5. Higher is better.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct HandScore(pub u64);

// Hand categories
const HIGH_CARD: u64 = 0;
const ONE_PAIR: u64 = 1;
const TWO_PAIR: u64 = 2;
const THREE_OF_A_KIND: u64 = 3;
const STRAIGHT: u64 = 4;
const FLUSH: u64 = 5;
const FULL_HOUSE: u64 = 6;
const FOUR_OF_A_KIND: u64 = 7;
const STRAIGHT_FLUSH: u64 = 8;

fn pack(cat: u64, t1: u64, t2: u64, t3: u64, t4: u64, t5: u64) -> HandScore {
    HandScore(
        (cat << 40)
            | (t1 << 32)
            | (t2 << 24)
            | (t3 << 16)
            | (t4 << 8)
            | t5,
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn eval_5(cards: &[Card; 5]) -> HandScore {
    let mut rank_counts = [0u8; 15]; // indexed 2..=14
    let mut suit_counts = [0u8; 4];

    for c in cards.iter() {
        rank_counts[c.rank.0 as usize] += 1;
        suit_counts[c.suit.0 as usize] += 1;
    }

    let is_flush = suit_counts[0] == 5
        || suit_counts[1] == 5
        || suit_counts[2] == 5
        || suit_counts[3] == 5;

    // Collect sorted ranks present ascending — stack allocated
    let mut ranks_present = [0u8; 5];
    let mut rp_len = 0usize;
    for r in 2u8..=14 {
        if rank_counts[r as usize] > 0 {
            ranks_present[rp_len] = r;
            rp_len += 1;
        }
    }

    let (is_straight, straight_high) = if rp_len == 5 {
        if ranks_present[4] - ranks_present[0] == 4 {
            (true, ranks_present[4])
        } else if ranks_present[0] == 2
            && ranks_present[1] == 3
            && ranks_present[2] == 4
            && ranks_present[3] == 5
            && ranks_present[4] == 14
        {
            (true, 5u8)
        } else {
            (false, 0u8)
        }
    } else {
        (false, 0u8)
    };

    // Build groups iterating ranks descending — all stack allocated
    let mut quads = [0u8; 1];
    let mut quad_len = 0usize;
    let mut trips = [0u8; 1];
    let mut trip_len = 0usize;
    let mut pairs = [0u8; 2];
    let mut pair_len = 0usize;
    let mut singles = [0u8; 5];
    let mut single_len = 0usize;

    for r in (2u8..=14).rev() {
        match rank_counts[r as usize] {
            4 => { quads[quad_len] = r; quad_len += 1; }
            3 => { trips[trip_len] = r; trip_len += 1; }
            2 => { pairs[pair_len] = r; pair_len += 1; }
            1 => { singles[single_len] = r; single_len += 1; }
            _ => {}
        }
    }

    if is_straight && is_flush {
        return pack(STRAIGHT_FLUSH, straight_high as u64, 0, 0, 0, 0);
    }
    if quad_len > 0 {
        let kicker = if single_len > 0 { singles[0] } else { 0 };
        return pack(FOUR_OF_A_KIND, quads[0] as u64, kicker as u64, 0, 0, 0);
    }
    if trip_len > 0 && pair_len > 0 {
        return pack(FULL_HOUSE, trips[0] as u64, pairs[0] as u64, 0, 0, 0);
    }
    if is_flush {
        // A flush means all 5 are singles (no pairs/trips/quads)
        return pack(
            FLUSH,
            singles[0] as u64,
            singles[1] as u64,
            singles[2] as u64,
            singles[3] as u64,
            singles[4] as u64,
        );
    }
    if is_straight {
        return pack(STRAIGHT, straight_high as u64, 0, 0, 0, 0);
    }
    if trip_len > 0 {
        let k1 = if single_len > 0 { singles[0] } else { 0 };
        let k2 = if single_len > 1 { singles[1] } else { 0 };
        return pack(THREE_OF_A_KIND, trips[0] as u64, k1 as u64, k2 as u64, 0, 0);
    }
    if pair_len >= 2 {
        let kicker = if single_len > 0 { singles[0] } else { 0 };
        return pack(
            TWO_PAIR,
            pairs[0] as u64,
            pairs[1] as u64,
            kicker as u64,
            0,
            0,
        );
    }
    if pair_len == 1 {
        let k1 = if single_len > 0 { singles[0] } else { 0 };
        let k2 = if single_len > 1 { singles[1] } else { 0 };
        let k3 = if single_len > 2 { singles[2] } else { 0 };
        return pack(ONE_PAIR, pairs[0] as u64, k1 as u64, k2 as u64, k3 as u64, 0);
    }
    pack(
        HIGH_CARD,
        singles[0] as u64,
        singles[1] as u64,
        singles[2] as u64,
        singles[3] as u64,
        singles[4] as u64,
    )
}

/// Find the best 5-card hand from 7 cards by trying all C(7,2)=21 combinations
/// of which 2 cards to skip. Used for correctness verification in tests.
#[cfg_attr(not(test), allow(dead_code))]
pub fn best_5_from_7(cards: &[Card; 7]) -> HandScore {
    let mut best = HandScore(0);
    for skip1 in 0..7 {
        for skip2 in (skip1 + 1)..7 {
            let mut five = [Card::default(); 5];
            let mut idx = 0;
            for i in 0..7 {
                if i != skip1 && i != skip2 {
                    five[idx] = cards[i];
                    idx += 1;
                }
            }
            let score = eval_5(&five);
            if score > best {
                best = score;
            }
        }
    }
    best
}

/// Returns highest-straight high card from a rank bitmask (bit r = rank r present).
/// Low ace (wheel A-2-3-4-5) returns 5. Returns 0 if no straight.
#[inline]
fn straight_high_from_mask(mask: u16) -> u8 {
    // Add bit 1 as low ace if ace (bit 14) is present
    let m = if mask & (1 << 14) != 0 { mask | 2 } else { mask };
    // Check from highest (14) down to wheel (5)
    for high in (5u8..=14).rev() {
        let lo = high - 4;
        let bits: u16 = 0b11111u16 << lo;
        if m & bits == bits {
            return high;
        }
    }
    0
}

/// Returns top 5 ranks from a bitmask, descending.
#[inline]
fn top5_from_mask(mask: u16) -> [u8; 5] {
    let mut result = [0u8; 5];
    let mut idx = 0usize;
    for r in (2u8..=14).rev() {
        if mask & (1u16 << r) != 0 {
            result[idx] = r;
            idx += 1;
            if idx == 5 {
                break;
            }
        }
    }
    result
}

/// Direct 7-card hand evaluator — single pass, no heap allocation, ~8-10x faster
/// than calling best_5_from_7. Used in the hot simulation loop.
pub fn eval_7(cards: &[Card; 7]) -> HandScore {
    let mut rank_counts = [0u8; 15];
    let mut suit_counts = [0u8; 4];
    let mut suit_rank_mask = [0u16; 4]; // bitmask of ranks present per suit

    for c in cards.iter() {
        let r = c.rank.0 as usize;
        let s = c.suit.0 as usize;
        rank_counts[r] += 1;
        suit_counts[s] += 1;
        suit_rank_mask[s] |= 1u16 << r;
    }

    // Check flush (any suit with >= 5 cards)
    let flush_suit = if suit_counts[0] >= 5 {
        Some(0usize)
    } else if suit_counts[1] >= 5 {
        Some(1)
    } else if suit_counts[2] >= 5 {
        Some(2)
    } else if suit_counts[3] >= 5 {
        Some(3)
    } else {
        None
    };

    if let Some(fs) = flush_suit {
        let fmask = suit_rank_mask[fs];
        let sf_high = straight_high_from_mask(fmask);
        if sf_high > 0 {
            return pack(STRAIGHT_FLUSH, sf_high as u64, 0, 0, 0, 0);
        }
        let fr = top5_from_mask(fmask);
        return pack(
            FLUSH,
            fr[0] as u64,
            fr[1] as u64,
            fr[2] as u64,
            fr[3] as u64,
            fr[4] as u64,
        );
    }

    // Build groups iterating descending (highest rank first)
    // With 7 cards: max 1 quad, 2 trips, 3 pairs, 7 singles
    let mut quads = [0u8; 1];
    let mut qlen = 0usize;
    let mut trips = [0u8; 2];
    let mut tlen = 0usize;
    let mut pairs = [0u8; 3];
    let mut plen = 0usize;
    let mut singles = [0u8; 7];
    let mut slen = 0usize;

    for r in (2u8..=14).rev() {
        match rank_counts[r as usize] {
            4 => {
                quads[qlen] = r;
                qlen += 1;
            }
            3 => {
                trips[tlen] = r;
                tlen += 1;
            }
            2 => {
                pairs[plen] = r;
                plen += 1;
            }
            1 => {
                singles[slen] = r;
                slen += 1;
            }
            _ => {}
        }
    }

    if qlen > 0 {
        // Kicker: highest rank not in the quad
        let t = if tlen > 0 { trips[0] } else { 0 };
        let p = if plen > 0 { pairs[0] } else { 0 };
        let s = if slen > 0 { singles[0] } else { 0 };
        let kicker = t.max(p).max(s);
        return pack(FOUR_OF_A_KIND, quads[0] as u64, kicker as u64, 0, 0, 0);
    }

    if tlen > 0 && (tlen > 1 || plen > 0) {
        // Full house: best trips + best available pair
        let pair_rank = if tlen > 1 { trips[1] } else { pairs[0] };
        return pack(FULL_HOUSE, trips[0] as u64, pair_rank as u64, 0, 0, 0);
    }

    // Straight check via rank bitmask
    let mut rank_mask = 0u16;
    for r in 2u8..=14 {
        if rank_counts[r as usize] > 0 {
            rank_mask |= 1u16 << r;
        }
    }
    let sh = straight_high_from_mask(rank_mask);
    if sh > 0 {
        return pack(STRAIGHT, sh as u64, 0, 0, 0, 0);
    }

    if tlen > 0 {
        // No full house → plen == 0 → remaining 4 are all singles
        let k1 = if slen > 0 { singles[0] } else { 0 };
        let k2 = if slen > 1 { singles[1] } else { 0 };
        return pack(THREE_OF_A_KIND, trips[0] as u64, k1 as u64, k2 as u64, 0, 0);
    }

    if plen >= 2 {
        // Kicker: best rank not used by top two pairs
        let from_third_pair = if plen >= 3 { pairs[2] } else { 0 };
        let from_singles = if slen > 0 { singles[0] } else { 0 };
        let kicker = from_third_pair.max(from_singles);
        return pack(
            TWO_PAIR,
            pairs[0] as u64,
            pairs[1] as u64,
            kicker as u64,
            0,
            0,
        );
    }

    if plen == 1 {
        let k1 = if slen > 0 { singles[0] } else { 0 };
        let k2 = if slen > 1 { singles[1] } else { 0 };
        let k3 = if slen > 2 { singles[2] } else { 0 };
        return pack(ONE_PAIR, pairs[0] as u64, k1 as u64, k2 as u64, k3 as u64, 0);
    }

    // High card: top 5 of 7 singles
    pack(
        HIGH_CARD,
        singles[0] as u64,
        singles[1] as u64,
        singles[2] as u64,
        singles[3] as u64,
        singles[4] as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;

    fn c(s: &str) -> Card {
        Card::parse(s).unwrap()
    }

    fn hand(cards: [&str; 5]) -> [Card; 5] {
        [c(cards[0]), c(cards[1]), c(cards[2]), c(cards[3]), c(cards[4])]
    }

    #[test]
    fn test_high_card() {
        let h = hand(["2h", "4d", "6c", "8s", "Th"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, HIGH_CARD);
    }

    #[test]
    fn test_one_pair() {
        let h = hand(["2h", "2d", "6c", "8s", "Th"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, ONE_PAIR);
    }

    #[test]
    fn test_two_pair() {
        let h = hand(["2h", "2d", "6c", "6s", "Th"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, TWO_PAIR);
    }

    #[test]
    fn test_three_of_a_kind() {
        let h = hand(["2h", "2d", "2c", "6s", "Th"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, THREE_OF_A_KIND);
    }

    #[test]
    fn test_straight() {
        let h = hand(["2h", "3d", "4c", "5s", "6h"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, STRAIGHT);
        // high card is 6
        assert_eq!((score.0 >> 32) & 0xff, 6);
    }

    #[test]
    fn test_wheel_straight() {
        let h = hand(["Ah", "2d", "3c", "4s", "5h"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, STRAIGHT);
        // wheel high is 5
        assert_eq!((score.0 >> 32) & 0xff, 5);
    }

    #[test]
    fn test_wheel_scores_lower_than_six_high() {
        let wheel = eval_5(&hand(["Ah", "2d", "3c", "4s", "5h"]));
        let six_high = eval_5(&hand(["2h", "3d", "4c", "5s", "6h"]));
        assert!(wheel < six_high);
    }

    #[test]
    fn test_flush() {
        let h = hand(["2h", "4h", "6h", "8h", "Th"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, FLUSH);
    }

    #[test]
    fn test_full_house() {
        let h = hand(["2h", "2d", "2c", "6s", "6h"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, FULL_HOUSE);
    }

    #[test]
    fn test_four_of_a_kind() {
        let h = hand(["2h", "2d", "2c", "2s", "6h"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, FOUR_OF_A_KIND);
    }

    #[test]
    fn test_straight_flush() {
        let h = hand(["2h", "3h", "4h", "5h", "6h"]);
        let score = eval_5(&h);
        assert_eq!(score.0 >> 40, STRAIGHT_FLUSH);
    }

    #[test]
    fn test_royal_flush_beats_straight_flush() {
        let royal = eval_5(&hand(["Th", "Jh", "Qh", "Kh", "Ah"]));
        let sf = eval_5(&hand(["2h", "3h", "4h", "5h", "6h"]));
        assert!(royal > sf);
    }

    #[test]
    fn test_hand_ordering() {
        let sf = eval_5(&hand(["2h", "3h", "4h", "5h", "6h"]));
        let quads = eval_5(&hand(["2h", "2d", "2c", "2s", "6h"]));
        let fh = eval_5(&hand(["2h", "2d", "2c", "6s", "6h"]));
        let flush = eval_5(&hand(["2h", "4h", "6h", "8h", "Th"]));
        let straight = eval_5(&hand(["2h", "3d", "4c", "5s", "6h"]));
        let trips = eval_5(&hand(["2h", "2d", "2c", "6s", "Th"]));
        let two_pair = eval_5(&hand(["2h", "2d", "6c", "6s", "Th"]));
        let one_pair = eval_5(&hand(["2h", "2d", "6c", "8s", "Th"]));
        let high_card = eval_5(&hand(["2h", "4d", "6c", "8s", "Th"]));

        assert!(sf > quads);
        assert!(quads > fh);
        assert!(fh > flush);
        assert!(flush > straight);
        assert!(straight > trips);
        assert!(trips > two_pair);
        assert!(two_pair > one_pair);
        assert!(one_pair > high_card);
    }

    #[test]
    fn test_best_5_from_7() {
        // Royal flush hidden in 7 cards
        let cards = [
            c("Th"), c("Jh"), c("Qh"), c("Kh"), c("Ah"),
            c("2d"), c("3c"),
        ];
        let score = best_5_from_7(&cards);
        assert_eq!(score.0 >> 40, STRAIGHT_FLUSH);
    }

    // eval_7 tests — verify it matches best_5_from_7 reference
    fn seven(cards: [&str; 7]) -> [Card; 7] {
        [c(cards[0]), c(cards[1]), c(cards[2]), c(cards[3]),
         c(cards[4]), c(cards[5]), c(cards[6])]
    }

    fn check_eval7(cards: [&str; 7]) {
        let h = seven(cards);
        assert_eq!(eval_7(&h), best_5_from_7(&h),
            "eval_7 mismatch for {:?}", cards);
    }

    #[test]
    fn test_eval7_straight_flush() {
        check_eval7(["2h", "3h", "4h", "5h", "6h", "Kd", "Ac"]);
    }

    #[test]
    fn test_eval7_royal_flush() {
        check_eval7(["Th", "Jh", "Qh", "Kh", "Ah", "2d", "3c"]);
    }

    #[test]
    fn test_eval7_four_of_a_kind() {
        check_eval7(["Ah", "Ad", "Ac", "As", "Kh", "Qd", "Jc"]);
    }

    #[test]
    fn test_eval7_full_house() {
        check_eval7(["Ah", "Ad", "Ac", "Kh", "Kd", "Qd", "Jc"]);
    }

    #[test]
    fn test_eval7_two_trips() {
        // Two trips — best full house uses higher trips
        check_eval7(["Ah", "Ad", "Ac", "Kh", "Kd", "Kc", "Qd"]);
    }

    #[test]
    fn test_eval7_flush() {
        check_eval7(["2h", "4h", "6h", "8h", "Th", "Kd", "Ac"]);
    }

    #[test]
    fn test_eval7_flush_six_cards() {
        check_eval7(["2h", "4h", "6h", "8h", "Th", "Kh", "Ac"]);
    }

    #[test]
    fn test_eval7_straight() {
        check_eval7(["2h", "3d", "4c", "5s", "6h", "Kd", "Ac"]);
    }

    #[test]
    fn test_eval7_wheel() {
        check_eval7(["Ah", "2d", "3c", "4s", "5h", "Kd", "Qc"]);
    }

    #[test]
    fn test_eval7_three_of_a_kind() {
        check_eval7(["Ah", "Ad", "Ac", "2s", "4h", "6d", "8c"]);
    }

    #[test]
    fn test_eval7_two_pair() {
        check_eval7(["Ah", "Ad", "Kh", "Kd", "Qh", "Jd", "Tc"]);
    }

    #[test]
    fn test_eval7_three_pairs() {
        check_eval7(["Ah", "Ad", "Kh", "Kd", "Qh", "Qd", "Jc"]);
    }

    #[test]
    fn test_eval7_one_pair() {
        check_eval7(["Ah", "Ad", "2h", "3d", "4c", "5s", "7h"]);
    }

    #[test]
    fn test_eval7_high_card() {
        check_eval7(["Ah", "Kd", "Qc", "Js", "9h", "7d", "2c"]);
    }

    #[test]
    fn test_eval7_straight_flush_vs_flush() {
        // 5 hearts including a straight — should be straight flush, not flush
        check_eval7(["6h", "7h", "8h", "9h", "Th", "2d", "3c"]);
    }

    #[test]
    fn test_eval7_wheel_straight_flush() {
        check_eval7(["Ah", "2h", "3h", "4h", "5h", "Kd", "Qc"]);
    }
}
