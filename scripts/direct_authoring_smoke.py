"""Real HTTP save/run persistence with synthetic device evidence.

Use worker device acceptance separately: this fixture cannot prove emulator behavior.
"""

from test_library_smoke import main

if __name__ == "__main__":
    main(direct=True)
