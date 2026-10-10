# Generated Binance Spot stream codec

This local dependency contains **Rust** code emitted by the official SbeTool
1.35.6. Java is required only to regenerate it; ordinary builds and runtime do
not invoke Java, download schemas, or execute a build script. The crate has no
third-party runtime dependencies. All four stream templates are generated;
Strait currently uses only `DepthDiffStreamEvent` (10003).

Sources:

- [Official generator](https://github.com/aeron-io/simple-binary-encoding/tree/1.35.6),
  [pinned JAR](https://repo.maven.apache.org/maven2/uk/co/real-logic/sbe-all/1.35.6/sbe-all-1.35.6.jar).
- [Binance stream schema 1:0](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/stream_1_0.xml),
  retained verbatim in `schema/stream_1_0.xml` (retrieved 2026-10-09).
- [Binance's Rust generation example](https://github.com/binance/binance-sbe-rust-sample-app).

SHA-256 pins are enforced by [the regeneration script](../../scripts/regenerate_binance_spot_sbe.py):

| Input | SHA-256 |
| --- | --- |
| SbeTool 1.35.6 JAR | `1b85c37264866bcb83cf82d9a8adcf0d7c5faad2e080e75c0813dc9f13c15783` |
| Retained XML schema | `6ea328467e144311b1f1efff38e9fe613829997f041dd02a3b7077885d10a1f7` |

The JAR's published Maven SHA-1 was also checked when downloaded:
`4bcb5b3fa009d7e32febc5254629eb3d760c92dd`.
The upstream generator license is retained in [LICENSE](LICENSE).

## Regeneration

Download the pinned JAR to a local/temporary directory, then from the repository root:

```sh
python3 scripts/regenerate_binance_spot_sbe.py --schema stream --jar /path/to/sbe-all-1.35.6.jar
python3 scripts/regenerate_binance_spot_sbe.py --schema stream --jar /path/to/sbe-all-1.35.6.jar --check
```

Use `--java /path/to/bin/java` for a temporary runtime. Java 21 was used locally.
The script checks input hashes before invoking Java, generates in a temporary
folder, and applies `rustfmt` (edition 2021). The only manifest adjustment is the
Apache-2.0 license metadata. Do not edit generated `.rs` files manually.
Changing a schema/tool pin requires reviewing the new inputs and repeating
correctness, malformed-input, dependency and benchmark checks.

SbeTool emits a warning about the uint32 trades count; this is Binance's official
schema definition. The depth groups use uint16 counts and are unaffected.

## Network input boundary

Generated accessors index slices and assume valid lengths. Their `Result` methods
primarily protect the parent-decoder lifecycle; they do not make arbitrary network
bytes safe. Strait first validates the complete supported header/root/group/symbol
layout and configured limits, then uses generated field/group accessors. Keep that
preflight and its malformed-input tests when updating the generator/schema.
The generated iterator's `wrapping_add` advances an internal sentinel index; wire
numbers still undergo checked exact domain conversion.

UTF-8, expected symbol, nonnegative timestamp/sequence, sequence range, price and
quantity signs/ranges/precision, borrowed-view lifetimes, buffer reuse and error
cleanup remain Strait's responsibility. Snapshot API schema 3:5 uses a [separate generated crate](../binance-api/README.md);
this crate covers stream schema 1:0 only.
