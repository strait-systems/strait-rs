"""Deterministic synthetic wire fixtures. Run from any directory with Python 3.

Independent encoding from the documented XML layouts, not Rust decoder internals.
These are not recorded exchange messages and do not establish live compatibility.
"""
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parent


def header(block, template, schema, version):
    return struct.pack("<HHHH", block, template, schema, version)


def levels(rows, count_type):
    return struct.pack("<H" + count_type, 16, len(rows)) + b"".join(
        struct.pack("<qq", price, quantity) for price, quantity in rows
    )


def text(value):
    encoded = value.encode("utf-8")
    return struct.pack("<B", len(encoded)) + encoded


def increment(bids, asks):
    return (
        header(26, 10003, 1, 0)
        + struct.pack("<qqqbb", 1700000000000000, 101, 103, -2, -3)
        + levels(bids, "H")
        + levels(asks, "H")
        + text("BTCUSDT")
    )


def wrapper(status, payload):
    return (
        header(3, 50, 3, 5)
        + struct.pack("<BH", 0, status)
        + struct.pack("<HH", 19, 1)
        + struct.pack("<BBBqq", 2, 1, 1, 6000, 250)
        + text("snap1")
        + struct.pack("<I", len(payload))
        + payload
    )


def snapshot(bids, asks):
    return wrapper(
        200,
        header(10, 200, 3, 5)
        + struct.pack("<qbb", 100, -2, -3)
        + levels(bids, "I")
        + levels(asks, "I"),
    )


ROOT.joinpath("depth.bin").write_bytes(
    increment([(6500000, 1250), (6499900, 0)], [(6500100, 2000)])
)
ROOT.joinpath("snapshot.bin").write_bytes(
    snapshot([(6500000, 1500), (6499900, 2500)], [(6500100, 2000)])
)
ROOT.joinpath("depth-128.bin").write_bytes(
    increment(
        [(6500000 - i, 1000 + i) for i in range(64)],
        [(6500100 + i, 2000 + i) for i in range(64)],
    )
)
ROOT.joinpath("snapshot-10000.bin").write_bytes(
    snapshot(
        [(6500000 - i, 1000 + i) for i in range(5000)],
        [(6500100 + i, 2000 + i) for i in range(5000)],
    )
)
MESSAGE = b"Too many requests"
ERROR = (
    header(18, 100, 3, 5)
    + struct.pack("<hqq", -1003, 1700000000000000, 1700000005000000)
    + struct.pack("<H", len(MESSAGE))
    + MESSAGE
    + struct.pack("<I", 0)
)
ROOT.joinpath("rate-limit.bin").write_bytes(wrapper(429, ERROR))
