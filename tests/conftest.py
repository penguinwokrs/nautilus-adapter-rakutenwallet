import os
import json
from pathlib import Path
import pytest


def has_api_keys() -> bool:
    """Check if Rakuten Wallet API keys are available."""
    return bool(
        os.environ.get("RAKUTENW_API_KEY")
        and os.environ.get("RAKUTENW_API_SECRET")
    )


def _check_rust_extension() -> bool:
    """Check if the Rust extension is importable."""
    try:
        import nautilus_rakutenwallet
        return hasattr(nautilus_rakutenwallet, 'rakutenw')
    except ImportError:
        return False


requires_api_keys = pytest.mark.skipif(
    not has_api_keys(),
    reason="Requires RAKUTENW_API_KEY and RAKUTENW_API_SECRET environment variables",
)

requires_rust_extension = pytest.mark.skipif(
    not _check_rust_extension(),
    reason="Requires compiled Rust extension (_nautilus_rakutenwallet)",
)

integration = pytest.mark.integration

CASSETTE_DIR = Path(__file__).parent / "cassettes"


@pytest.fixture
def vcr(request):
    """VCR-style fixture for recording/replaying HTTP responses."""
    record = request.config.getoption("--record-cassettes", default=False)

    cassette_name = request.node.name.replace("[", "_").replace("]", "_").rstrip("_")
    cassette_path = CASSETTE_DIR / f"{cassette_name}.json"

    def play_or_record(live_fn):
        if cassette_path.exists() and not record:
            data = json.loads(cassette_path.read_text())
            return json.dumps(data)
        if not record:
            pytest.skip(f"Cassette not found: {cassette_name}.json")
        result = live_fn()
        CASSETTE_DIR.mkdir(parents=True, exist_ok=True)
        cassette_path.write_text(json.dumps(json.loads(result), indent=2))
        return result

    return play_or_record


def pytest_addoption(parser):
    parser.addoption(
        "--record-cassettes",
        action="store_true",
        default=False,
        help="Record new cassettes (makes live API calls)",
    )
