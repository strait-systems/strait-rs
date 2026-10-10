# Spot synthetic SBE fixtures

Run `python3 tests/fixtures/binance/spot/generate.py` from the repository root.
The script has no dependencies, network use, RNG or timestamps from the host.
It recreates all five binary files deterministically from the XML layouts linked in
[protocol documentation](../../../../docs/spot-sbe.md).

These are synthetic, not exchange recordings. Small fixtures encode:

- BTCUSDT update range [101,103], event UTC 1700000000000000us.
- Price exponent -2, quantity exponent -3.
- Increment bids (6500000,1250), (6499900,0); ask (6500100,2000).
- Snapshot update ID 100, bids (6500000,1500), (6499900,2500); ask (6500100,2000).
- Wrapper request ID `snap1`, status 200 (or 429), one request-weight/minute entry
  with limit 6000/count 250. These are fixture values, not configured admission budgets.
- Error -1003, message `Too many requests`, absolute retry time 1700000005000000us.

Large fixtures have 64 levels per side for increments and 5000 per side for
snapshots. Prices remain ordered and quantities positive. Tests assert explicit
expected values independently of decoding, and fuzz smoke exercises truncation,
byte corruption, bad layouts, identity mismatches and over-limit payloads.
