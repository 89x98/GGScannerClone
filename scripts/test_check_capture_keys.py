import base64
import contextlib
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import check_capture_keys as checker


def table(identifier="1", byte=0):
    return json.dumps({identifier: base64.b64encode(bytes([byte]) * 4096).decode()}).encode()


class KeyCheckTests(unittest.TestCase):
    def test_missing_changed_and_retained_historical_keys(self):
        self.assertEqual(checker.compare_keys({"1": b"a", "2": b"old"},
                                             {"1": b"b", "3": b"new"}), (["3"], ["1"]))
        self.assertEqual(checker.compare_keys({"1": b"a", "2": b"old"}, {"1": b"a"}), ([], []))

    def test_validates_identifiers_encoding_length_and_duplicates(self):
        for data in [b"[]", b"{}", b'{"1": "bad!"}', b'{"1": "AAAA"}',
                     b'{"1": 4}', table("65536"), table("01"), b'{"1": "", "1": ""}']:
            with self.subTest(data=data[:35]), self.assertRaises(ValueError):
                checker.parse_keys(data, 0xFFFF)
        self.assertEqual(len(checker.parse_keys(table("2920436228"), 0xFFFFFFFF)["2920436228"]), 4096)

    def test_checks_outgoing_commit_instead_of_working_tree(self):
        sha = "a" * 40
        urls = []

        def download(url):
            urls.append(url)
            return json.dumps({"sha": sha}).encode() if url.endswith("/master") else table()

        with patch.object(checker, "download", side_effect=download), \
                patch.object(checker.subprocess, "run", return_value=subprocess.CompletedProcess(
                    [], 0, stdout=table(), stderr=b"")) as git, \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(checker.check(Path("."), ["outgoing", "outgoing"]), 0)
        self.assertEqual(git.call_count, 2)
        self.assertTrue(all(call.args[0][2].startswith("outgoing:") for call in git.call_args_list))
        self.assertTrue(all(f"/{sha}/keys/" in url for url in urls[1:]))

    def test_outdated_keys_block_without_writing_files(self):
        with patch.object(checker, "download", side_effect=[
            json.dumps({"sha": "a" * 40}).encode(), table("2"), table("2")]), \
                patch.object(checker, "read_local", return_value=table()), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(checker.check(Path("."), []), 1)
        self.assertIn("missing IDs=['2']", output.getvalue())

    def test_unavailable_upstream_blocks(self):
        with patch.object(checker.sys, "argv", ["check_capture_keys.py"]), \
                patch.object(checker, "download", side_effect=OSError("offline")), \
                contextlib.redirect_stderr(io.StringIO()) as output:
            self.assertEqual(checker.main(), 1)
        self.assertIn("verification failed", output.getvalue())


if __name__ == "__main__":
    unittest.main()
