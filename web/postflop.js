// Post-flop hand evaluator and equity calculator
// Ported from src/eval.rs and src/sim.rs

const Postflop = (() => {
  // Card: { rank: 2-14, suit: 0-3 }
  // Rank: 2=2, 3=3, ..., T=10, J=11, Q=12, K=13, A=14
  // Suit: 0=h, 1=d, 2=c, 3=s

  const RANK_MAP = {
    '2': 2, '3': 3, '4': 4, '5': 5, '6': 6, '7': 7, '8': 8, '9': 9,
    'T': 10, 'J': 11, 'Q': 12, 'K': 13, 'A': 14
  };
  const SUIT_MAP = { 'h': 0, 'd': 1, 'c': 2, 's': 3 };

  function parseCard(str) {
    return { rank: RANK_MAP[str[0].toUpperCase()], suit: SUIT_MAP[str[1].toLowerCase()] };
  }

  function cardKey(c) {
    return c.rank * 4 + c.suit;
  }

  function fullDeck() {
    const deck = [];
    for (let suit = 0; suit < 4; suit++) {
      for (let rank = 2; rank <= 14; rank++) {
        deck.push({ rank, suit });
      }
    }
    return deck;
  }

  // Hand categories (higher = better)
  const HIGH_CARD = 0;
  const ONE_PAIR = 1;
  const TWO_PAIR = 2;
  const THREE_OF_A_KIND = 3;
  const STRAIGHT = 4;
  const FLUSH = 5;
  const FULL_HOUSE = 6;
  const FOUR_OF_A_KIND = 7;
  const STRAIGHT_FLUSH = 8;

  // Pack score into a single JS number (48 bits, within safe integer range)
  // Use multiplication instead of bitwise shift (JS bitwise ops truncate to 32 bits)
  const P40 = 1099511627776; // 2^40
  const P32 = 4294967296;    // 2^32
  const P24 = 16777216;      // 2^24
  const P16 = 65536;         // 2^16
  const P8  = 256;           // 2^8

  function pack(cat, t1, t2, t3, t4, t5) {
    return cat * P40 + t1 * P32 + t2 * P24 + t3 * P16 + t4 * P8 + t5;
  }

  // Find highest straight from a rank bitmask. Returns high card rank, or 0.
  // Handles wheel (A-2-3-4-5) returning 5.
  function straightHighFromMask(mask) {
    // Add bit 1 as low ace if ace (bit 14) is present
    const m = (mask & (1 << 14)) ? (mask | 2) : mask;
    for (let high = 14; high >= 5; high--) {
      const lo = high - 4;
      const bits = 0x1F << lo; // 5 consecutive bits
      if ((m & bits) === bits) return high;
    }
    return 0;
  }

  // Return top 5 ranks from bitmask, descending
  function top5FromMask(mask) {
    const result = [0, 0, 0, 0, 0];
    let idx = 0;
    for (let r = 14; r >= 2; r--) {
      if (mask & (1 << r)) {
        result[idx++] = r;
        if (idx === 5) break;
      }
    }
    return result;
  }

  // Direct 7-card hand evaluator — single pass, mirrors Rust eval_7
  function eval7(cards) {
    const rankCounts = new Uint8Array(15); // indexed 0-14, use 2-14
    const suitCounts = new Uint8Array(4);
    const suitRankMask = new Uint16Array(4);

    for (let i = 0; i < 7; i++) {
      const r = cards[i].rank;
      const s = cards[i].suit;
      rankCounts[r]++;
      suitCounts[s]++;
      suitRankMask[s] |= (1 << r);
    }

    // Check flush (any suit with >= 5 cards)
    let flushSuit = -1;
    if (suitCounts[0] >= 5) flushSuit = 0;
    else if (suitCounts[1] >= 5) flushSuit = 1;
    else if (suitCounts[2] >= 5) flushSuit = 2;
    else if (suitCounts[3] >= 5) flushSuit = 3;

    if (flushSuit >= 0) {
      const fmask = suitRankMask[flushSuit];
      const sfHigh = straightHighFromMask(fmask);
      if (sfHigh > 0) return pack(STRAIGHT_FLUSH, sfHigh, 0, 0, 0, 0);
      const fr = top5FromMask(fmask);
      return pack(FLUSH, fr[0], fr[1], fr[2], fr[3], fr[4]);
    }

    // Build groups iterating descending (highest rank first)
    const quads = [0];
    let qlen = 0;
    const trips = [0, 0];
    let tlen = 0;
    const pairs = [0, 0, 0];
    let plen = 0;
    const singles = [0, 0, 0, 0, 0, 0, 0];
    let slen = 0;

    for (let r = 14; r >= 2; r--) {
      switch (rankCounts[r]) {
        case 4: quads[qlen++] = r; break;
        case 3: trips[tlen++] = r; break;
        case 2: pairs[plen++] = r; break;
        case 1: singles[slen++] = r; break;
      }
    }

    if (qlen > 0) {
      const t = tlen > 0 ? trips[0] : 0;
      const p = plen > 0 ? pairs[0] : 0;
      const s = slen > 0 ? singles[0] : 0;
      const kicker = Math.max(t, p, s);
      return pack(FOUR_OF_A_KIND, quads[0], kicker, 0, 0, 0);
    }

    if (tlen > 0 && (tlen > 1 || plen > 0)) {
      const pairRank = tlen > 1 ? trips[1] : pairs[0];
      return pack(FULL_HOUSE, trips[0], pairRank, 0, 0, 0);
    }

    // Straight check via rank bitmask
    let rankMask = 0;
    for (let r = 2; r <= 14; r++) {
      if (rankCounts[r] > 0) rankMask |= (1 << r);
    }
    const sh = straightHighFromMask(rankMask);
    if (sh > 0) return pack(STRAIGHT, sh, 0, 0, 0, 0);

    if (tlen > 0) {
      const k1 = slen > 0 ? singles[0] : 0;
      const k2 = slen > 1 ? singles[1] : 0;
      return pack(THREE_OF_A_KIND, trips[0], k1, k2, 0, 0);
    }

    if (plen >= 2) {
      const fromThirdPair = plen >= 3 ? pairs[2] : 0;
      const fromSingles = slen > 0 ? singles[0] : 0;
      const kicker = Math.max(fromThirdPair, fromSingles);
      return pack(TWO_PAIR, pairs[0], pairs[1], kicker, 0, 0);
    }

    if (plen === 1) {
      const k1 = slen > 0 ? singles[0] : 0;
      const k2 = slen > 1 ? singles[1] : 0;
      const k3 = slen > 2 ? singles[2] : 0;
      return pack(ONE_PAIR, pairs[0], k1, k2, k3, 0);
    }

    return pack(HIGH_CARD, singles[0], singles[1], singles[2], singles[3], singles[4]);
  }

  // Calculate post-flop equity
  // hand1, hand2: arrays of 2 card strings (e.g. ["Ah", "Kd"])
  // board: array of 0-5 card strings (e.g. ["Qc", "Jh", "5s"])
  // Returns: { wins1, wins2, ties, total, pct1, pct2, tiePct }
  function calculateEquity(hand1Strs, hand2Strs, boardStrs) {
    const h1 = hand1Strs.map(parseCard);
    const h2 = hand2Strs.map(parseCard);
    const board = boardStrs.map(parseCard);

    // Build set of used card keys
    const usedKeys = new Set();
    for (const c of h1) usedKeys.add(cardKey(c));
    for (const c of h2) usedKeys.add(cardKey(c));
    for (const c of board) usedKeys.add(cardKey(c));

    // Remaining deck
    const remaining = fullDeck().filter(c => !usedKeys.has(cardKey(c)));

    const needed = 5 - board.length;
    let wins1 = 0, wins2 = 0, ties = 0, total = 0;

    // Reusable 7-card arrays to avoid allocation in hot loop
    const seven1 = new Array(7);
    const seven2 = new Array(7);
    seven1[0] = h1[0]; seven1[1] = h1[1];
    seven2[0] = h2[0]; seven2[1] = h2[1];

    // Copy fixed board cards
    for (let i = 0; i < board.length; i++) {
      seven1[2 + i] = board[i];
      seven2[2 + i] = board[i];
    }

    if (needed === 0) {
      // All 5 community cards known — single evaluation
      const s1 = eval7(seven1);
      const s2 = eval7(seven2);
      if (s1 > s2) wins1 = 1;
      else if (s1 < s2) wins2 = 1;
      else ties = 1;
      total = 1;
    } else if (needed === 1) {
      // Turn or river unknown — enumerate remaining cards
      const n = remaining.length;
      for (let i = 0; i < n; i++) {
        seven1[6] = remaining[i];
        seven2[6] = remaining[i];
        const s1 = eval7(seven1);
        const s2 = eval7(seven2);
        if (s1 > s2) wins1++;
        else if (s1 < s2) wins2++;
        else ties++;
        total++;
      }
    } else if (needed === 2) {
      // Two unknown — enumerate C(remaining, 2) combinations
      const n = remaining.length;
      for (let i = 0; i < n - 1; i++) {
        seven1[5] = remaining[i];
        seven2[5] = remaining[i];
        for (let j = i + 1; j < n; j++) {
          seven1[6] = remaining[j];
          seven2[6] = remaining[j];
          const s1 = eval7(seven1);
          const s2 = eval7(seven2);
          if (s1 > s2) wins1++;
          else if (s1 < s2) wins2++;
          else ties++;
          total++;
        }
      }
    } else {
      // 3+ unknown cards — too many combinations for browser JS
      return null;
    }

    return {
      wins1, wins2, ties, total,
      pct1: total > 0 ? (wins1 / total * 100) : 0,
      pct2: total > 0 ? (wins2 / total * 100) : 0,
      tiePct: total > 0 ? (ties / total * 100) : 0,
    };
  }

  return { parseCard, fullDeck, cardKey, eval7, calculateEquity };
})();
