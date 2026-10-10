#!/usr/bin/env python3
"""Regenerate/check Spot Rust codecs using pinned official SbeTool (no runtime Java)."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
JAR_SHA256 = "1b85c37264866bcb83cf82d9a8adcf0d7c5faad2e080e75c0813dc9f13c15783"
SCHEMAS = {
    "stream": ("binance-stream", "stream_1_0.xml", "spot_stream", "6ea328467e144311b1f1efff38e9fe613829997f041dd02a3b7077885d10a1f7", None),
    "api": ("binance-api", "spot_3_5.xml", "spot_sbe", "542776a038883dafe962341a041323ff5d8d3e36f1b860ab60156a88a4080bcf", {50, 100, 200}),
}


def verify(path, expected):
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise SystemExit(f"Unexpected SHA-256 for {path}; review source/version before updating pins")


def generate(args, name):
    directory, filename, package, schema_hash, templates = SCHEMAS[name]
    dest = ROOT / "generated" / directory
    schema = dest / "schema" / filename
    verify(schema, schema_hash)
    with tempfile.TemporaryDirectory(prefix="strait-sbe-") as temp:
        if templates is not None:
            # Retain official field/type definitions verbatim in meaning; only
            # omit unrelated message templates. No handwritten schema fields.
            tree = ET.parse(schema)
            namespace = "http://fixprotocol.io/2016/sbe"
            ET.register_namespace("sbe", namespace)
            ET.register_namespace("mbx", "https://developers.binance.com/docs/binance-spot-api-docs")
            found = set()
            for message in list(tree.getroot().findall(f"{{{namespace}}}message")):
                template = int(message.attrib["id"])
                if template in templates:
                    found.add(template)
                else:
                    tree.getroot().remove(message)
            if found != templates:
                raise SystemExit(f"Missing selected templates: {templates - found}")
            schema = Path(temp) / filename
            tree.write(schema, encoding="UTF-8", xml_declaration=True)
        subprocess.run([
            args.java, f"-Dsbe.output.dir={temp}", "-Dsbe.target.language=Rust",
            "-Dsbe.rust.crate.version=0.1.0", "-jar", str(args.jar.resolve()), str(schema),
        ], check=True)
        output = Path(temp) / package
        manifest = output / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace(
            'edition = "2021"', 'edition = "2021"\nlicense = "Apache-2.0"'))
        subprocess.run(["rustfmt", "--edition", "2021", *map(str, sorted((output / "src").glob("*.rs")))], check=True)
        files = [Path("Cargo.toml"), *sorted(p.relative_to(output) for p in (output / "src").glob("*.rs"))]
        extra = set(p.name for p in (dest / "src").glob("*.rs")) - set(p.name for p in (output / "src").glob("*.rs"))
        if args.check:
            differences = [str(p) for p in files if not (dest / p).exists() or (dest / p).read_bytes() != (output / p).read_bytes()]
            if differences or extra:
                raise SystemExit(f"{name} output differs: {differences}; extra files: {sorted(extra)}")
            print(f"Generated {name} crate matches pinned SbeTool output")
        else:
            # Replace only generator-owned source/manifest, preserving provenance.
            for filename in extra:
                (dest / "src" / filename).unlink()
            shutil.copytree(output / "src", dest / "src", dirs_exist_ok=True)
            shutil.copyfile(manifest, dest / "Cargo.toml")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, required=True, help="sbe-all-1.35.6.jar")
    parser.add_argument("--java", default="java", help="Java executable (temporary runtime supported)")
    parser.add_argument("--schema", choices=["all", *SCHEMAS], default="all")
    parser.add_argument("--check", action="store_true", help="Compare without modifying retained output")
    args = parser.parse_args()
    verify(args.jar, JAR_SHA256)
    for name in SCHEMAS if args.schema == "all" else [args.schema]:
        generate(args, name)


if __name__ == "__main__":
    main()
