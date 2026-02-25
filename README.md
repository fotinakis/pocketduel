# pocketduel

Texas Hold'em preflop equity calculator. Give it two hole card holdings and it tells you exactly how often each hand wins.

Uses exhaustive enumeration over all 1,712,304 possible 5-card boards — no Monte Carlo approximation, just exact math.

## Usage

### Single matchup

```
pocketduel --hand1 <HAND> --hand2 <HAND>
```

Three input formats are supported (case-insensitive, mixable):

| Format | Example | Description |
|--------|---------|-------------|
| Shorthand suited | `AKs` | Ace-King suited — suits chosen automatically |
| Shorthand offsuit | `KQo` | King-Queen offsuit — suits chosen automatically |
| Shorthand pair | `AA` | Pocket aces — suits chosen automatically |
| Explicit | `AhKh` | Ace of hearts, King of hearts |
| Explicit spaced | `"Ah Kh"` | Same, space-separated |

Rank characters: `A` `K` `Q` `J` `T` `9`–`2`. Suit characters: `h` `d` `c` `s`.

```bash
# Shorthand — suits chosen automatically, shared ranks resolved without conflict
pocketduel --hand1 AA --hand2 KK
pocketduel --hand1 AKs --hand2 QQ
pocketduel --hand1 AKo --hand2 KQo

# Explicit suits
pocketduel --hand1 AhAd --hand2 KhKd
pocketduel --hand1 "Ah Kh" --hand2 "Qd Qc"

# Formats can be mixed
pocketduel --hand1 AKs --hand2 QdQc
```

Example output:

```
Running exhaustive simulation (1,712,304 boards)...

Results:
  Hand 1 (AhAd) wins:  1410336 (82.36%)
  Hand 2 (KhKd) wins:   292660 (17.09%)
  Ties:                   9308  (0.54%)
  Total boards:        1712304
```

### Full equity table

```
pocketduel --everything [--output-dir <DIR>]
```

Computes all **14,365 unique preflop matchups** (every pair from the 169 canonical hand types) and writes one JSON file per matchup to `--output-dir` (default: `./equity_data`).

```bash
pocketduel --everything                          # writes to ./equity_data/
pocketduel --everything --output-dir ./my_data   # custom directory
```

The run takes ~48 minutes in release mode. It is **resumable**: re-running skips any matchup whose output file already exists and is up to date. Interrupt with Ctrl-C at any time and pick up where you left off.

Each output file is named `{hand1}_vs_{hand2}.json` in canonical order (e.g. `AA_vs_KK.json`, `AKs_vs_QQ.json`) and contains:

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

The `version` field enables forward compatibility: if a future release adds new fields to the schema, bumping the version number will cause only stale files to be recomputed on the next run.

## Build

Requires Rust (install via [rustup](https://rustup.rs)).

```bash
cargo build --release
./target/release/pocketduel --hand1 AhAd --hand2 KhKd
```

Single matchups run in ~0.2 seconds on modern hardware.

## How it works

**Enumeration:** Iterates all C(48,5) = 1,712,304 combinations of 5 community cards from the 48 remaining deck cards (52 minus the 4 hole cards). Every possible board is evaluated exactly once.

**Hand evaluation:** Each player's best 5-card hand is found from their 7 available cards (2 hole + 5 community) using a direct single-pass evaluator. Hands are scored as a packed `u64` — integer comparison gives the correct winner with no special-case logic.

**Hand categories** (low to high): high card, one pair, two pair, three of a kind, straight, flush, full house, four of a kind, straight flush.

## Running tests

```bash
cargo test -- --skip sim::tests --skip everything::tests::test_equity_spot_checks
                              # fast unit tests, ~0s
cargo test -- --skip sim::tests
                              # includes equity spot-checks, ~5s release
cargo test                    # full suite including simulation integration tests
```
