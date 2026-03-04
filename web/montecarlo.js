// Multiway equity calculator: exact enumeration + Monte Carlo estimation.
// Depends on Postflop.parseCard, Postflop.fullDeck, Postflop.cardKey.
// Includes a zero-allocation eval7 for Monte Carlo sampling speed.

const Multiway = (() => {
  const { parseCard, fullDeck, cardKey } = Postflop;

  // ── Hand evaluation constants (mirrors postflop.js) ───────────────────────

  const HIGH_CARD = 0, ONE_PAIR = 1, TWO_PAIR = 2, THREE_KIND = 3;
  const STRAIGHT = 4, FLUSH = 5, FULL_HOUSE = 6, FOUR_KIND = 7, STR_FLUSH = 8;
  const P40 = 1099511627776, P32 = 4294967296, P24 = 16777216, P16 = 65536, P8 = 256;

  function pack(cat, a, b, c, d, e) {
    return cat * P40 + a * P32 + b * P24 + c * P16 + d * P8 + e;
  }

  function straightHigh(mask) {
    const m = (mask & (1 << 14)) ? (mask | 2) : mask;
    for (let h = 14; h >= 5; h--) {
      const bits = 0x1F << (h - 4);
      if ((m & bits) === bits) return h;
    }
    return 0;
  }

  // ── Pre-allocated buffers (zero-allocation eval7) ─────────────────────────

  const _rc = new Uint8Array(15);
  const _sc = new Uint8Array(4);
  const _sm = new Uint16Array(4);
  const _top = [0, 0, 0, 0, 0];
  // Groups: max 1 quad, 2 trips, 3 pairs, 7 singles in 7 cards
  const _q = [0], _t = [0, 0], _p = [0, 0, 0], _s = [0, 0, 0, 0, 0, 0, 0];
  let _ql, _tl, _pl, _sl;

  function top5(mask) {
    let n = 0;
    for (let i = 14; i >= 2 && n < 5; i--) if (mask & (1 << i)) _top[n++] = i;
    return _top;
  }

  function eval7(cards) {
    _rc.fill(0); _sc.fill(0); _sm.fill(0);
    for (let i = 0; i < 7; i++) {
      const r = cards[i].rank, s = cards[i].suit;
      _rc[r]++; _sc[s]++; _sm[s] |= (1 << r);
    }

    // Flush
    let fs = -1;
    if (_sc[0] >= 5) fs = 0; else if (_sc[1] >= 5) fs = 1;
    else if (_sc[2] >= 5) fs = 2; else if (_sc[3] >= 5) fs = 3;
    if (fs >= 0) {
      const fm = _sm[fs], sh = straightHigh(fm);
      if (sh > 0) return pack(STR_FLUSH, sh, 0, 0, 0, 0);
      const f = top5(fm);
      return pack(FLUSH, f[0], f[1], f[2], f[3], f[4]);
    }

    // Build groups descending
    _ql = 0; _tl = 0; _pl = 0; _sl = 0;
    for (let r = 14; r >= 2; r--) {
      switch (_rc[r]) {
        case 4: _q[_ql++] = r; break;
        case 3: _t[_tl++] = r; break;
        case 2: _p[_pl++] = r; break;
        case 1: _s[_sl++] = r; break;
      }
    }

    if (_ql > 0) {
      const k = Math.max(_tl > 0 ? _t[0] : 0, _pl > 0 ? _p[0] : 0, _sl > 0 ? _s[0] : 0);
      return pack(FOUR_KIND, _q[0], k, 0, 0, 0);
    }
    if (_tl > 0 && (_tl > 1 || _pl > 0)) {
      return pack(FULL_HOUSE, _t[0], _tl > 1 ? _t[1] : _p[0], 0, 0, 0);
    }

    let rm = 0;
    for (let r = 2; r <= 14; r++) if (_rc[r] > 0) rm |= (1 << r);
    const sh = straightHigh(rm);
    if (sh > 0) return pack(STRAIGHT, sh, 0, 0, 0, 0);

    if (_tl > 0) return pack(THREE_KIND, _t[0], _sl > 0 ? _s[0] : 0, _sl > 1 ? _s[1] : 0, 0, 0);
    if (_pl >= 2) {
      const k = Math.max(_pl >= 3 ? _p[2] : 0, _sl > 0 ? _s[0] : 0);
      return pack(TWO_PAIR, _p[0], _p[1], k, 0, 0);
    }
    if (_pl === 1) return pack(ONE_PAIR, _p[0], _sl > 0 ? _s[0] : 0, _sl > 1 ? _s[1] : 0, _sl > 2 ? _s[2] : 0, 0);
    return pack(HIGH_CARD, _s[0], _s[1], _s[2], _s[3], _s[4]);
  }

  // ── Shared setup for both exact and Monte Carlo ───────────────────────────

  function setup(handStrs, boardStrs) {
    const hands = handStrs.map(h => h.map(parseCard));
    const board = boardStrs.map(parseCard);
    const numHands = hands.length;

    const usedKeys = new Set();
    for (const h of hands) for (const c of h) usedKeys.add(cardKey(c));
    for (const c of board) usedKeys.add(cardKey(c));

    const remaining = fullDeck().filter(c => !usedKeys.has(cardKey(c)));
    const needed = 5 - board.length;

    // Pre-allocate 7-card arrays for each hand
    const sevens = hands.map(h => {
      const arr = new Array(7);
      arr[0] = h[0]; arr[1] = h[1];
      for (let i = 0; i < board.length; i++) arr[2 + i] = board[i];
      return arr;
    });

    return { numHands, remaining, needed, sevens };
  }

  // Score all hands, distribute equity share.
  // splitEquitySum[p] accumulates each player's equity from split boards.
  // splitCountSum[p] accumulates how many boards player p was involved in a split.
  function scoreRound(sevens, numHands, equitySum, splitEquitySum, splitCountSum) {
    let maxScore = -1;
    const scores = new Array(numHands);
    for (let p = 0; p < numHands; p++) {
      scores[p] = eval7(sevens[p]);
      if (scores[p] > maxScore) maxScore = scores[p];
    }
    let nw = 0;
    for (let p = 0; p < numHands; p++) if (scores[p] === maxScore) nw++;
    const isSplit = nw > 1;
    const share = 1.0 / nw;
    for (let p = 0; p < numHands; p++) {
      if (scores[p] === maxScore) {
        equitySum[p] += share;
        if (isSplit) {
          splitEquitySum[p] += share;
          splitCountSum[p]++;
        }
      }
    }
    return isSplit;
  }

  // ── Exact enumeration (needed ≤ 2) ────────────────────────────────────────

  function calculateExact(handStrs, boardStrs) {
    const { numHands, remaining, needed, sevens } = setup(handStrs, boardStrs);
    const equity = new Float64Array(numHands);
    const splitEquity = new Float64Array(numHands);
    const splitCount = new Float64Array(numHands);
    let splits = 0, total = 0;
    const n = remaining.length;

    if (needed === 0) {
      if (scoreRound(sevens, numHands, equity, splitEquity, splitCount)) splits++;
      total = 1;

    } else if (needed === 1) {
      for (let i = 0; i < n; i++) {
        for (let p = 0; p < numHands; p++) sevens[p][6] = remaining[i];
        if (scoreRound(sevens, numHands, equity, splitEquity, splitCount)) splits++;
        total++;
      }

    } else if (needed === 2) {
      for (let i = 0; i < n - 1; i++) {
        for (let p = 0; p < numHands; p++) sevens[p][5] = remaining[i];
        for (let j = i + 1; j < n; j++) {
          for (let p = 0; p < numHands; p++) sevens[p][6] = remaining[j];
          if (scoreRound(sevens, numHands, equity, splitEquity, splitCount)) splits++;
          total++;
        }
      }
    }

    const equities = [], splitEquities = [], splitFreqs = [];
    for (let p = 0; p < numHands; p++) {
      equities.push(equity[p] / total * 100);
      splitEquities.push(splitEquity[p] / total * 100);
      splitFreqs.push(splitCount[p] / total * 100);
    }

    return {
      equities,
      splitEquities,
      splitFreqs,
      splitPct: total > 0 ? splits / total * 100 : 0,
      total,
      margin: 0,
      isExact: true,
      done: true
    };
  }

  // ── Monte Carlo estimation (needed ≥ 3) ───────────────────────────────────

  const MC_SAMPLES = 100000;
  const MC_CHUNK   = 2500;

  let _runId = 0;

  function run(handStrs, boardStrs, onUpdate) {
    const id = ++_runId;
    const { numHands, remaining, needed, sevens } = setup(handStrs, boardStrs);
    const boardLen = 5 - needed;
    const equity = new Float64Array(numHands);
    const splitEquity = new Float64Array(numHands);
    const splitCount = new Float64Array(numHands);
    let splits = 0, total = 0;

    function sample() {
      // Fisher-Yates partial shuffle: draw `needed` random cards
      for (let i = 0; i < needed; i++) {
        const j = i + (Math.random() * (remaining.length - i)) | 0;
        const tmp = remaining[i]; remaining[i] = remaining[j]; remaining[j] = tmp;
        for (let p = 0; p < numHands; p++) sevens[p][2 + boardLen + i] = remaining[i];
      }
      if (scoreRound(sevens, numHands, equity, splitEquity, splitCount)) splits++;
      total++;
    }

    function chunk() {
      if (id !== _runId) return; // cancelled

      const end = Math.min(total + MC_CHUNK, MC_SAMPLES);
      while (total < end) sample();

      const equities = [], splitEquities = [], splitFreqs = [];
      for (let p = 0; p < numHands; p++) {
        equities.push(equity[p] / total * 100);
        splitEquities.push(splitEquity[p] / total * 100);
        splitFreqs.push(splitCount[p] / total * 100);
      }

      // Margin of error: conservative worst-case (p=0.5)
      const margin = 1.96 * Math.sqrt(0.25 / total) * 100;

      onUpdate({
        equities,
        splitEquities,
        splitFreqs,
        splitPct: splits / total * 100,
        total,
        margin,
        isExact: false,
        done: total >= MC_SAMPLES
      });

      if (total < MC_SAMPLES) setTimeout(chunk, 0);
    }

    setTimeout(chunk, 0);
    return id;
  }

  function cancel() { _runId++; }

  return { calculateExact, run, cancel };
})();
