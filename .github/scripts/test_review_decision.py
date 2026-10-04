import tempfile
import unittest
from pathlib import Path
import importlib.util

spec = importlib.util.spec_from_file_location(
    "review_gate", Path(__file__).with_name("check-review-decision.py")
)
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ReviewDispositionTests(unittest.TestCase):
    def evaluate(self, report):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.md"
            path.write_text(report, encoding="utf-8")
            return gate.check_report(path)

    def test_clean_report(self):
        self.assertEqual(self.evaluate(
            "Reviewed documentation only.\nSTRAIT_REVIEW_DECISION=no_demonstrated_findings\n"
        ), "no_demonstrated_findings")

    def test_required_changes_and_missing_evidence_fail(self):
        for status in ("changes_required", "evidence_required", "inconclusive", "unknown"):
            with self.subTest(status=status), self.assertRaises(ValueError):
                self.evaluate(f"STRAIT_REVIEW_DECISION={status}\n")

    def test_missing_ambiguous_and_malformed_reports_fail(self):
        good = "STRAIT_REVIEW_DECISION=no_demonstrated_findings"
        for report in ("", "No findings", f"`{good}`", f"{good}\n{good}",
                       f"{good}\nSTRAIT_REVIEW_DECISION=changes-required",
                       f"{good}\nUnreviewed trailing text"):
            with self.subTest(report=report), self.assertRaises(ValueError):
                self.evaluate(report)

    def test_absent_report_fails(self):
        with self.assertRaises(OSError):
            gate.check_report("/path/to/absent/strait-review-report")


if __name__ == "__main__":
    unittest.main()
