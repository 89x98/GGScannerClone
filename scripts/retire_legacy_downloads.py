"""Retire only the temporary Windows aliases from renamed releases."""

import argparse
from datetime import date, datetime, timezone
import json
from pathlib import Path
import subprocess


def retirement_targets(releases, policy, today):
    cutoff = date.fromisoformat(policy["legacyDownloadsUntil"])
    if today < cutoff:
        return []
    first = datetime.strptime(policy["firstRenamedTag"], "v%Y.%m.%d").date()
    targets = []
    for release in releases:
        tag = release["tag_name"]
        try:
            published = datetime.strptime(tag, "v%Y.%m.%d").date()
        except ValueError:
            continue
        if release.get("draft") or not first <= published < cutoff:
            continue
        for asset in release.get("assets", []):
            if asset["name"] in ("GOODCapture.exe", "GOODScanner.exe"):
                targets.append((tag, asset["name"]))
    return targets


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    policy = json.loads(
        (Path(__file__).resolve().parents[1] / ".github/release-policy.json").read_text()
    )
    today = datetime.now(timezone.utc).date()
    if today < date.fromisoformat(policy["legacyDownloadsUntil"]):
        print("Compatibility downloads are still within their one-month window.")
        return
    pages = json.loads(subprocess.check_output(
        ["gh", "api", "--paginate", "--slurp", f"repos/{args.repo}/releases"], text=True
    ))
    for tag, asset in retirement_targets(
        [release for page in pages for release in page], policy, today
    ):
        print(f"Retire {tag}/{asset}")
        if not args.dry_run:
            subprocess.run(
                ["gh", "release", "delete-asset", tag, asset, "--yes", "--repo", args.repo],
                check=True,
            )


if __name__ == "__main__":
    main()
