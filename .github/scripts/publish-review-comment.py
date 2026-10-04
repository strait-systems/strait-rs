"""Publish a review artifact as data using a separate, scoped GitHub token."""
import json
import os
from pathlib import Path
import subprocess
import sys

MARKER = "<!-- strait-ai-review -->"
BOT = "github-actions[bot]"


def api(method, endpoint, payload=None):
    command = ["gh", "api", "--method", method, endpoint]
    if payload is not None:
        command.extend(["--input", "-"])
    result = subprocess.run(
        command, input=json.dumps(payload) if payload is not None else None,
        text=True, capture_output=True, check=True,
    )
    return json.loads(result.stdout)


def render_comment(report, head, run_url):
    header = (
        f"{MARKER}\n## Strait AI review\n\n"
        f"Reviewed commit: `{head}` · [Full report and workflow]({run_url})\n\n"
        "AI findings are advisory evidence; tests and performance gates still apply.\n\n"
    )
    # Avoid unsolicited user/team notifications from model-generated text.
    report = report.replace("@", "&#64;")
    encoded = report.encode("utf-8")
    if len(encoded) > 55000:
        report = encoded[:55000].decode("utf-8", errors="ignore")
        report += "\n\n_Report truncated. Read the complete artifact linked above._"
    return header + report


def publish(repository, number, head, body, request=api):
    root = f"repos/{repository}"
    number = int(number)
    pr = request("GET", f"{root}/pulls/{number}")
    if (pr["state"] != "open" or pr["draft"]
            or pr["base"]["ref"] not in ("dev", "main")
            or pr["head"]["repo"]["full_name"] != repository
            or pr["head"]["sha"] != head):
        return "Skipped: PR no longer eligible or report is stale."
    existing = None
    page = 1
    while True:
        comments = request(
            "GET", f"{root}/issues/{number}/comments?per_page=100&page={page}"
        )
        for comment in comments:
            if (comment["user"]["login"] == BOT
                    and comment["body"].startswith(MARKER)):
                existing = comment["id"]
                break
        if existing is not None or len(comments) < 100:
            break
        page += 1
    # Recheck after pagination, which may take time on a busy PR.
    latest = request("GET", f"{root}/pulls/{number}")
    if (latest["head"]["sha"] != head or latest["state"] != "open"
            or latest["draft"] or latest["base"]["ref"] not in ("dev", "main")):
        return "Skipped: PR changed while preparing comment."
    if existing is None:
        request("POST", f"{root}/issues/{number}/comments", {"body": body})
    else:
        request("PATCH", f"{root}/issues/comments/{existing}", {"body": body})
    return "AI review comment published."


if __name__ == "__main__":
    repository = os.environ["GITHUB_REPOSITORY"]
    head = os.environ["REVIEW_HEAD"]
    run_url = (
        f"{os.environ['GITHUB_SERVER_URL']}/{repository}/actions/runs/"
        f"{os.environ['GITHUB_RUN_ID']}/attempts/{os.environ['GITHUB_RUN_ATTEMPT']}"
    )
    report = Path(sys.argv[1]).read_text(encoding="utf-8")
    print(publish(repository, os.environ["REVIEW_PR"], head,
                  render_comment(report, head, run_url)))
