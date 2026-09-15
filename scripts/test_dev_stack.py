"""Supervisor policy checks use synthetic settings/process identities, never Doppler."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import dev_stack


class DevStackTests(unittest.TestCase):
    def test_secret_environment_is_not_inherited(self):
        with patch.dict(
            "os.environ",
            {
                "HOME": "/home/dev",
                "PATH": "/bin",
                "OPENAI_API_KEY": "synthetic",
                "DATABASE_URL": "synthetic",
                "MOBILE_QA_WORKER_TOKEN": "synthetic",
                "VITE_SECRET": "synthetic",
            },
            clear=True,
        ):
            self.assertEqual(
                dev_stack.clean_env(), {"HOME": "/home/dev", "PATH": "/bin"}
            )

    def test_stale_pid_does_not_signal_anyone(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / "state.json"
            state.write_text(json.dumps({"pid": 123, "started": "old"}))
            with (
                patch.object(dev_stack, "STATE", state),
                patch.object(dev_stack, "identity", return_value="new"),
                patch("os.kill") as kill,
            ):
                with self.assertRaisesRegex(RuntimeError, "Stale supervisor"):
                    dev_stack.stop()
                kill.assert_not_called()

    def test_config_refuses_secret_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / "config.json"
            config.write_text(json.dumps({"token": "synthetic"}))
            with self.assertRaisesRegex(ValueError, "only"):
                dev_stack.settings(config)

    def test_doppler_only_wraps_requested_child(self):
        command = dev_stack.doppler("project", "dev", ["worker"])
        self.assertEqual(command[-2:], ["--", "worker"])
        self.assertIn("--no-fallback", command)
        self.assertIn("--forward-signals", command)


if __name__ == "__main__":
    unittest.main()
