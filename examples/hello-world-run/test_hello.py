"""Unit tests for the hello-world CLI (M-1 and M-2)."""

import subprocess
import sys
import unittest
from pathlib import Path

from hello import greet

ROOT = Path(__file__).resolve().parent


class GreetFunctionTests(unittest.TestCase):
    def test_T002_greet_returns_default_greeting(self):
        """M-1 SC-001: greet() returns exactly 'Hello, world!'."""
        self.assertEqual(greet(), "Hello, world!")

    def test_T002_greet_returns_named_greeting(self):
        """M-2 SC-001: greet('Ada') returns 'Hello, Ada!' verbatim."""
        self.assertEqual(greet("Ada"), "Hello, Ada!")


class CliTests(unittest.TestCase):
    def test_T001_cli_prints_default_greeting(self):
        """FR-001: no arguments prints 'Hello, world!' plus newline."""
        result = subprocess.run(
            [sys.executable, str(ROOT / "hello.py")],
            capture_output=True, text=True, check=True,
        )
        self.assertEqual(result.stdout, "Hello, world!\n")

    def test_T001_cli_prints_named_greeting(self):
        """M-2 FR-001: one name argument prints 'Hello, Ada!' plus newline."""
        result = subprocess.run(
            [sys.executable, str(ROOT / "hello.py"), "Ada"],
            capture_output=True, text=True, check=True,
        )
        self.assertEqual(result.stdout, "Hello, Ada!\n")


if __name__ == "__main__":
    unittest.main()
