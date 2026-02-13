import pytest


class TestRakutenwDataClientConfig:
    def test_valid_config(self):
        from nautilus_rakutenwallet.config import RakutenwDataClientConfig
        config = RakutenwDataClientConfig(
            api_key="test_key",
            api_secret="test_secret",
        )
        assert config.api_key == "test_key"
        assert config.api_secret == "test_secret"
        assert config.timeout_ms == 10000
        assert config.order_book_depth == 20

    def test_missing_api_key_raises(self):
        from nautilus_rakutenwallet.config import RakutenwDataClientConfig
        with pytest.raises(ValueError, match="requires both api_key and api_secret"):
            RakutenwDataClientConfig(api_key=None, api_secret="secret")

    def test_missing_api_secret_raises(self):
        from nautilus_rakutenwallet.config import RakutenwDataClientConfig
        with pytest.raises(ValueError, match="requires both api_key and api_secret"):
            RakutenwDataClientConfig(api_key="key", api_secret=None)

    def test_custom_timeout(self):
        from nautilus_rakutenwallet.config import RakutenwDataClientConfig
        config = RakutenwDataClientConfig(
            api_key="key",
            api_secret="secret",
            timeout_ms=5000,
        )
        assert config.timeout_ms == 5000


class TestRakutenwExecClientConfig:
    def test_valid_config(self):
        from nautilus_rakutenwallet.config import RakutenwExecClientConfig
        config = RakutenwExecClientConfig(
            api_key="test_key",
            api_secret="test_secret",
        )
        assert config.api_key == "test_key"
        assert config.api_secret == "test_secret"
        assert config.order_poll_interval_sec == 1.0

    def test_custom_poll_interval(self):
        from nautilus_rakutenwallet.config import RakutenwExecClientConfig
        config = RakutenwExecClientConfig(
            api_key="key",
            api_secret="secret",
            order_poll_interval_sec=0.5,
        )
        assert config.order_poll_interval_sec == 0.5

    def test_missing_credentials_raises(self):
        from nautilus_rakutenwallet.config import RakutenwExecClientConfig
        with pytest.raises(ValueError):
            RakutenwExecClientConfig(api_key=None, api_secret=None)
