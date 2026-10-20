# Generated Binance Spot API snapshot codec

This dependency-free local Rust crate is official SbeTool 1.35.6 output for API
schema 3:5. Java is used only to regenerate it; normal builds and runtime need no
Java, network download or build script.

The original [Binance schema](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/spot_3_5.xml)
is retained in `schema/spot_3_5.xml`, retrieved on 2026-10-09 through the
[public repository CDN](https://cdn.jsdelivr.net/gh/binance/binance-spot-api-docs@master/sbe/schemas/spot_3_5.xml)
after direct GitHub connections timed out. Its SHA-256 is
`542776a038883dafe962341a041323ff5d8d3e36f1b860ab60156a88a4080bcf`.

[The generation script](../../scripts/regenerate_binance_spot_sbe.py) verifies the original
schema and generator hashes, retains official types, and selects only message
50 (`WebSocketResponse`), 100 (`ErrorResponse`) and 200 (`DepthResponse`). It changes
no selected field definitions/layouts. Unrelated trading/API message templates are
omitted before generation. Generated Rust is formatted; the manifest adds
Apache-2.0 license metadata. The generator license is retained in [LICENSE](LICENSE).

From the repository root:

```sh
python3 scripts/regenerate_binance_spot_sbe.py --schema api --jar /path/to/sbe-all-1.35.6.jar
python3 scripts/regenerate_binance_spot_sbe.py --schema api --jar /path/to/sbe-all-1.35.6.jar --check
```

Without `--schema`, both stream and API crates are generated/checked. Pass
`--java /path/to/bin/java` to use a temporary Java runtime. Generator download and
hash are documented [beside the stream crate](../binance-stream/README.md).

Do not edit generated Rust by hand. The generator warns that the official depth
schema uses uint32 group counts; this matches the retained schema and is not changed.

Generated accessors assume valid bounds and can panic on arbitrary input. Strait
validates the complete wrapper/depth layout before accessing it. Error responses
validate their fixed root and each variable-field span before the corresponding
generated accessor. It checks UTF-8, enum values, request identity, numeric signs/scales/precision,
snapshot ordering, and error timestamps. Only validated immutable response views
expose the rate-limit iterator, which uses generated group accessors. Keep these
validation gates and malformed-input tests when updating the schema/tool pins.
