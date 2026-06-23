from pathlib import Path
import importlib.util
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "validate_repository.py"


def load_guard():
    spec = importlib.util.spec_from_file_location("validate_repository", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class RepositoryGuardTests(unittest.TestCase):
    def test_required_public_surface_mentions_readme_and_security(self):
        guard = load_guard()
        self.assertIn("README.md", guard.REQUIRED)
        self.assertIn("SECURITY.md", guard.REQUIRED)
        self.assertIn("assets/hero.svg", guard.REQUIRED)

    def test_required_public_surface_tracks_backend_env_example(self):
        guard = load_guard()
        self.assertIn(".env.example", guard.REQUIRED)

    def test_guard_blocks_generated_database_artifacts(self):
        guard = load_guard()
        patterns = "\n".join(guard.FORBIDDEN_PATH_PATTERNS)
        self.assertIn("sqlite", patterns)
        self.assertIn("turso", patterns)


if __name__ == "__main__":
    unittest.main()
