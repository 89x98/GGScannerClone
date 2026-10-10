"""Read-only check of bundled keys against one immutable StarDB revision."""

import argparse
import base64
import binascii
import json
from pathlib import Path
import re
import subprocess
import sys
import urllib.error
import urllib.request


SOURCE_REPOSITORY = "juliuskreutz/stardb-exporter"
GAME_FILES = {
    "gi": ("genshin/keys/gi.json", 0xFFFF),
    "hsr": ("experimental/hsr/keys/hsr.json", 0xFFFFFFFF),
}
MAX_DOWNLOAD_BYTES = 2 * 1024 * 1024


def download(url):
    request = urllib.request.Request(url, headers={"User-Agent": "GOODScanner-key-check"})
    with urllib.request.urlopen(request, timeout=15) as response:
        data = response.read(MAX_DOWNLOAD_BYTES + 1)
    if len(data) > MAX_DOWNLOAD_BYTES:
        raise ValueError("upstream response exceeds the size limit")
    return data


def unique_object(pairs):
    result = {}
    for name, value in pairs:
        if name in result:
            raise ValueError("duplicate JSON entry")
        result[name] = value
    return result


def parse_keys(data, max_identifier):
    document = json.loads(data, object_pairs_hook=unique_object)
    if not isinstance(document, dict) or not document:
        raise ValueError("key table must be a nonempty JSON object")
    decoded = {}
    for identifier, encoded in document.items():
        if not re.fullmatch(r"0|[1-9][0-9]*", identifier) or int(identifier) > max_identifier:
            raise ValueError("key table contains an invalid identifier")
        if not isinstance(encoded, str):
            raise ValueError(f"key {identifier} must be a base64 string")
        try:
            key = base64.b64decode(encoded, validate=True)
        except (ValueError, binascii.Error) as error:
            raise ValueError(f"key {identifier} contains invalid base64") from error
        if len(key) != 4096:
            raise ValueError(f"key {identifier} must decode to 4096 bytes")
        decoded[identifier] = key
    return decoded


def compare_keys(local, upstream):
    # Keep historical local keys even if upstream stops distributing them.
    missing = sorted(upstream.keys() - local.keys(), key=int)
    changed = sorted((key for key in upstream.keys() & local.keys()
                      if upstream[key] != local[key]), key=int)
    return missing, changed


def read_local(root, path, revision):
    if revision is None:
        return (root / path).read_bytes()
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=root,
        capture_output=True, check=False,
    )
    if result.returncode:
        raise ValueError(f"cannot read {path} from Git revision {revision}")
    return result.stdout


def check(root, revisions):
    metadata = json.loads(download(
        f"https://api.github.com/repos/{SOURCE_REPOSITORY}/commits/master"))
    if not isinstance(metadata, dict):
        raise ValueError("upstream commit metadata must be an object")
    source_revision = metadata.get("sha")
    if not isinstance(source_revision, str) or not re.fullmatch(r"[0-9a-f]{40}", source_revision):
        raise ValueError("upstream did not return a valid commit identifier")
    upstream = {
        game: parse_keys(download(
            f"https://raw.githubusercontent.com/{SOURCE_REPOSITORY}/{source_revision}/keys/{game}.json"
        ), max_identifier)
        for game, (_, max_identifier) in GAME_FILES.items()
    }
    print(f"Checking capture keys against {SOURCE_REPOSITORY}@{source_revision[:12]}")
    outdated = False
    for revision in dict.fromkeys(revisions or [None]):
        label = revision or "working tree"
        for game, (path, max_identifier) in GAME_FILES.items():
            local = parse_keys(read_local(root, path, revision), max_identifier)
            missing, changed = compare_keys(local, upstream[game])
            if missing or changed:
                outdated = True
                print(f"OUTDATED {label} {path}: missing IDs={missing}; changed IDs={changed}")
            else:
                print(f"OK {label} {path} ({len(local)} bundled keys)")
    if outdated:
        print("Update the bundled key tables, validate and commit them before pushing. "
              "Key freshness does not verify inventory protocol compatibility.")
    return 1 if outdated else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", action="append", help="Git tree to check; repeatable")
    args = parser.parse_args()
    try:
        return check(Path(__file__).resolve().parent.parent, args.revision)
    except (OSError, ValueError, urllib.error.URLError) as error:
        print(f"Capture key verification failed: {error}. Push/build must wait until verification succeeds.",
              file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
