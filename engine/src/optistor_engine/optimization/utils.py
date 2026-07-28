"""Utility helpers for the optimization engine."""

import os
import sys


class HiddenPrints:
    """Context manager that suppresses stdout for the duration of the block.

    GEKKO's solver prints verbose diagnostics by default; this silences it
    when the caller (e.g. a rolling-horizon loop) doesn't want that noise.
    """

    def __enter__(self):
        self._original_stdout = sys.stdout
        sys.stdout = open(os.devnull, "w")

    def __exit__(self, exc_type, exc_val, exc_tb):
        sys.stdout.close()
        sys.stdout = self._original_stdout
