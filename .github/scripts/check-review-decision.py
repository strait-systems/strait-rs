"""Fail closed on missing, ambiguous, or negative AI review disposition."""
import re
import sys
from pathlib import Path


def check_report(path):
    report = Path(path).read_text(encoding="utf-8")
    decisions = re.findall(r"^STRAIT_REVIEW_DECISION=([a-z_]+)$", report, re.MULTILINE)
    if (len(decisions) != 1 or report.count("STRAIT_REVIEW_DECISION=") != 1
            or report.rstrip().splitlines()[-1] != f"STRAIT_REVIEW_DECISION={decisions[0]}"):
        raise ValueError("Expected exactly one review decision; inspect the artifact.")
    decision = decisions[0]
    if decision != "no_demonstrated_findings":
        raise ValueError(f"Review disposition: {decision}; inspect the artifact.")
    return decision


if __name__ == "__main__":
    try:
        print(check_report(sys.argv[1]))
    except (OSError, ValueError, IndexError) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)
