from typing import Optional
from nautilus_trader.config import LiveDataClientConfig, LiveExecClientConfig


class RakutenwDataClientConfig(LiveDataClientConfig):
    api_key: Optional[str] = None
    api_secret: Optional[str] = None
    timeout_ms: int = 10000
    proxy_url: Optional[str] = None
    order_book_depth: int = 20
    rate_limit_per_sec: Optional[float] = None  # REST API rate limit (default: 5.0 = 200ms interval)
    ws_rate_limit_per_sec: Optional[float] = None  # WS subscription rate (default: 0.5)

    def __post_init__(self):
        if not self.api_key or not self.api_secret:
            raise ValueError("RakutenwDataClientConfig requires both api_key and api_secret")


class RakutenwExecClientConfig(LiveExecClientConfig):
    api_key: Optional[str] = None
    api_secret: Optional[str] = None
    timeout_ms: int = 10000
    proxy_url: Optional[str] = None
    rate_limit_per_sec: Optional[float] = None  # REST API rate limit (default: 5.0 = 200ms interval)
    order_poll_interval_sec: float = 1.0  # Order status polling interval in seconds

    def __post_init__(self):
        if not self.api_key or not self.api_secret:
            raise ValueError("RakutenwExecClientConfig requires both api_key and api_secret")
