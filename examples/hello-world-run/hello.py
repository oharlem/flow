"""A minimal hello-world CLI: greets the world or a named user."""

import sys


def greet(name="world"):
    """Return the greeting for *name* (M-1 default, M-2 named)."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(greet(sys.argv[1]) if len(sys.argv) > 1 else greet())
