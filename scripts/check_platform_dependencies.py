"""Check edition/platform dependency isolation without compiling either target."""
import argparse
import re
import subprocess
from pathlib import Path


def canonical_dependency(line):
    line = re.sub(r" \([^)]*\)", "", line)
    # These two local switches became unconditional target-selected backends.
    # Exclude only their names when comparing with pre-cleanup main; every
    # third-party dependency and feature must still match exactly.
    if line.split()[0] in {"yas_core", "genshin_scanner"}:
        package, features = line.split(" features=", 1)
        retired = {"capturer_wgc", "capturer_screenshots"}
        line = package + " features=" + ",".join(f for f in features.split(",") if f not in retired)
    return line


def tree(root, target, capture):
    command = ["cargo", "tree", "--locked", "-p", "good_tools_app", "--target", target,
               "-e", "normal,build", "--prefix", "none", "--format", "{p} features={f}"]
    if capture:
        command += ["--features", "capture"]
    output = subprocess.check_output(command, cwd=root, text=True, encoding="utf-8")
    # Local checkout paths and tree deduplication markers are not dependencies.
    return {canonical_dependency(line) for line in output.splitlines()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-root", type=Path, help="clean main checkout for exact Windows comparison")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    linux_only = {"fontdb", "evdev", "xkeysym", "x11rb", "x11rb-protocol", "zbus"}
    for capture in (False, True):
        windows = tree(root, "x86_64-pc-windows-msvc", capture)
        linux = tree(root, "x86_64-unknown-linux-gnu", capture)
        windows_names = {line.split()[0] for line in windows}
        linux_names = {line.split()[0] for line in linux}
        assert not (windows_names & linux_only), f"Linux dependencies leaked into Windows: {windows_names & linux_only}"
        assert linux_only <= linux_names, f"Missing Linux dependencies: {linux_only - linux_names}"
        assert "pktmon" not in linux_names, "Windows packet capture leaked into Linux"
        assert ("pktmon" in windows_names) == capture, "Windows edition capture mapping changed"
        if args.baseline_root:
            baseline = tree(args.baseline_root, "x86_64-pc-windows-msvc", capture)
            assert windows == baseline, f"Windows dependency/features changed: added={windows - baseline}, removed={baseline - windows}"
        print(f"{'capture' if capture else 'OCR'}: Windows/Linux isolation passed")


if __name__ == "__main__":
    main()
