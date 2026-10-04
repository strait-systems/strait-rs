import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "publisher", Path(__file__).with_name("publish-review-comment.py")
)
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class ReviewCommentTests(unittest.TestCase):
    def exercise(self, comments=None, change=None, latest_change=None):
        pr = {"state": "open", "draft": False, "base": {"ref": "dev"},
              "head": {"sha": "abc", "repo": {"full_name": "owner/repo"}}}
        if change:
            change(pr)
        calls = []
        reads = 0

        def request(method, endpoint, payload=None):
            nonlocal reads
            calls.append((method, endpoint, payload))
            if "/pulls/" in endpoint:
                reads += 1
                result = copy.deepcopy(pr)
                if reads > 1 and latest_change:
                    latest_change(result)
                return result
            if method == "GET":
                page = int(endpoint.rsplit("=", 1)[1])
                return (comments or {}).get(page, [])
            return {}

        publisher.publish("owner/repo", 3, "abc", "report", request)
        return [call for call in calls if call[0] != "GET"]

    def test_create_and_update_only_owned_comment(self):
        self.assertEqual(self.exercise()[0][0], "POST")
        human = {"id": 1, "body": publisher.MARKER, "user": {"login": "human"}}
        bot = {"id": 2, "body": publisher.MARKER, "user": {"login": publisher.BOT}}
        writes = self.exercise({1: [human] * 100, 2: [bot]})
        self.assertEqual(writes, [("PATCH", "repos/owner/repo/issues/comments/2",
                                   {"body": "report"})])
        self.assertEqual(self.exercise({1: [human]})[0][0], "POST")

    def test_ineligible_or_stale_pr_never_writes(self):
        changes = [lambda p: p.update(state="closed"),
                   lambda p: p.update(draft=True),
                   lambda p: p["base"].update(ref="feature"),
                   lambda p: p["head"].update(sha="new"),
                   lambda p: p["head"]["repo"].update(full_name="fork/repo")]
        for change in changes:
            with self.subTest(change=change):
                self.assertEqual(self.exercise(change=change), [])
        for change in changes[:4]:
            with self.subTest(latest_change=change):
                self.assertEqual(self.exercise(latest_change=change), [])

    def test_size_limit_notifications_and_report_link(self):
        body = publisher.render_comment("@team " + "价" * 60000, "abc", "https://example/run")
        self.assertLess(len(body.encode("utf-8")), 60000)
        self.assertNotIn("@team", body)
        self.assertIn("https://example/run", body)
        self.assertIn("Report truncated", body)
        self.assertTrue(body.startswith(publisher.MARKER))


if __name__ == "__main__":
    unittest.main()
