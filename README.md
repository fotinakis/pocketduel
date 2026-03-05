# PocketDuel

https://pocketduel.com

A Texas Hold'em equity calculator for the browser. Pick 2–4 hole card holdings and a board, and it tells you in real time how often each hand wins — pre-flop through the river, heads-up or multi-way.

## Goal

The goal is a fast, accurate, zero-dependency poker equity tool that works on any device without an account, a download, or a backend. Everything runs client-side in plain JavaScript.

| Heads-up | Multi-way |
|:---:|:---:|
| <img width="400" alt="pocketduel-2-way" src="https://github.com/user-attachments/assets/369ae63d-20c2-4073-80d4-4cf38271e03b" /> |  <img width="400" alt="pocketduel-3-way" src="https://github.com/user-attachments/assets/ac515b25-8e70-41fa-8c00-4e23fe463275" />  |

## Architecture

The project has two halves: a Rust CLI that pre-computes preflop data, and a static web frontend that uses it.

### Rust CLI (`src/`)

| File | Role |
|------|------|
| `src/eval.rs` | Hand evaluator. Scores any 7-card hand as a packed `u64` so the winner is found by integer comparison — no special-case logic. Covers all standard hand categories from high card to straight flush. |
| `src/sim.rs` | Two-player equity engine. Enumerates all C(48,5) = 1,712,304 possible 5-card boards and counts wins, losses, and ties exactly. |
| `src/everything.rs` | Batch runner. Computes all 14,365 unique preflop head-up matchups (every pair from 169 canonical hand types) and writes one JSON file per matchup. Resumable: skips files that already exist. |
| `src/card.rs` | Card parsing and canonical hand representation. |
| `src/main.rs` | CLI entry point. |

The batch run takes ~48 minutes in release mode and produces ~14k JSON files in `web/equity_data/`. These are static assets — once generated they never change.

### Web frontend (`web/`)

| File | Role |
|------|------|
| `web/index.html` | Everything: markup, styles, and application logic in one file. Manages card selection UI, board state, routing between calculation modes, and the equity bar display. |
| `web/postflop.js` | Post-flop equity for 2-player matchups. Ports the Rust evaluator to JavaScript and enumerates remaining boards exactly — fast enough to run synchronously for 1- and 2-unknown-card cases, and C(45,3) ≈ 15k boards for the flop. |
| `web/montecarlo.js` | Multi-way equity (3–4 players). Exact enumeration grows too large for 3+ hands, so this runs 100,000 Monte Carlo samples spread across `setTimeout` chunks so the UI stays responsive. Reports a ±margin of error (95% CI). |
| `web/equity_data/` | Pre-computed preflop JSON files from the Rust batch run, fetched on demand when a heads-up pre-flop matchup is selected. |

### Why this split

- **Preflop heads-up**: Exact math, pre-computed. Instant lookup, zero client CPU.
- **Postflop heads-up**: Exact enumeration in JS. Fast enough to be synchronous (≤ ~15k boards on the flop, 44 on the turn, 1 on the river).
- **Multi-way (any street)**: Monte Carlo in JS. Exact enumeration over 3+ hands × remaining boards is too large for the browser, so sampling gives a close answer fast with a visible margin of error.

The 2-player exact path always wins on accuracy; Monte Carlo only kicks in when there are 3 or more players.

## Building the Rust CLI

Requires Rust (install via [rustup](https://rustup.rs)).

```bash
cargo build --release
./target/release/pocketduel --hand1 AhAd --hand2 KhKd
```

### Single matchup

```bash
pocketduel --hand1 AA --hand2 KK
pocketduel --hand1 AKs --hand2 QQ
pocketduel --hand1 AKo --hand2 KQo
```

Three input formats are supported (case-insensitive, mixable):

| Format | Example | Description |
|--------|---------|-------------|
| Shorthand suited | `AKs` | Ace-King suited — suits chosen automatically |
| Shorthand offsuit | `KQo` | King-Queen offsuit — suits chosen automatically |
| Shorthand pair | `AA` | Pocket aces — suits chosen automatically |
| Explicit | `AhKh` | Ace of hearts, King of hearts |
| Explicit spaced | `"Ah Kh"` | Same, space-separated |

Example output:

```
Running exhaustive simulation (1,712,304 boards)...

Results:
  Hand 1 (AhAd) wins:  1410336 (82.36%)
  Hand 2 (KhKd) wins:   292660 (17.09%)
  Ties:                   9308  (0.54%)
  Total boards:        1712304
```

### Pre-compute all preflop matchups

```bash
pocketduel --everything                          # writes to ./equity_data/
pocketduel --everything --output-dir ./my_data   # custom directory
```

Computes all **14,365 unique preflop matchups** and writes one JSON file per matchup. The run takes ~48 minutes in release mode and is **resumable** — re-running skips any matchup whose output file already exists.

Each file is named `{hand1}_vs_{hand2}.json` (e.g. `AA_vs_KK.json`) and contains:

```json
{
  "version": 1,
  "hand1": "AA",
  "hand2": "KK",
  "hand1_resolved": "AhAd",
  "hand2_resolved": "KhKd",
  "wins1": 1410336,
  "wins2": 292660,
  "ties": 9308,
  "total": 1712304,
  "win_pct1": 82.36,
  "win_pct2": 17.09,
  "tie_pct": 0.54
}
```

## Running tests

```bash
cargo test -- --skip sim::tests --skip everything::tests::test_equity_spot_checks
                              # fast unit tests, ~0s
cargo test -- --skip sim::tests
                              # includes equity spot-checks, ~5s release
cargo test                    # full suite including simulation integration tests
```

## Running the web app locally

```bash
npx serve web/
```
