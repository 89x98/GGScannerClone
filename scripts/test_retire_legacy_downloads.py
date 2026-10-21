from datetime import date
import unittest

from retire_legacy_downloads import retirement_targets


class RetirementTests(unittest.TestCase):
    def test_only_transition_aliases_are_removed_after_the_cutoff(self):
        policy = {"firstRenamedTag": "v2026.10.21", "legacyDownloadsUntil": "2026-11-21"}
        assets = [{"name": name} for name in (
            "GOODScanner.exe", "GOODCapture.exe", "GGScanner.exe", "GGScannerOCR.exe", "update.json"
        )]
        releases = [
            {"tag_name": tag, "assets": assets}
            for tag in ("v2026.10.16", "v2026.10.21", "v2026.11.20", "v2026.11.21", "data-bundle")
        ]
        releases.append({"tag_name": "v2026.10.22", "assets": assets, "draft": True})
        self.assertEqual(retirement_targets(releases, policy, date(2026, 11, 20)), [])
        self.assertEqual(retirement_targets(releases, policy, date(2026, 11, 21)), [
            ("v2026.10.21", "GOODScanner.exe"), ("v2026.10.21", "GOODCapture.exe"),
            ("v2026.11.20", "GOODScanner.exe"), ("v2026.11.20", "GOODCapture.exe"),
        ])


if __name__ == "__main__":
    unittest.main()
