# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2024 Penguinworks. All rights reserved.
#
#  Licensed under the GNU General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/gpl-3.0.en.html
# -------------------------------------------------------------------------------------------------
"""
Rakuten Wallet adapter constants.
"""

from nautilus_trader.model.identifiers import Venue


# Venue identifier
RAKUTENW_VENUE = Venue("RAKUTENW")

# API endpoints
RAKUTENW_BASE_URL = "https://exchange.rakuten-wallet.co.jp"
RAKUTENW_REST_URL = "https://exchange.rakuten-wallet.co.jp/api/v1"

# WebSocket endpoint
RAKUTENW_WS_URL = "wss://exchange.rakuten-wallet.co.jp/ws"

# Rate limits (200ms between requests = 5 req/s)
RAKUTENW_RATE_LIMIT = 5.0

# Order status mappings (Rakuten Wallet -> NautilusTrader string)
ORDER_STATUS_MAP = {
    "WORKING_ORDER": "ACCEPTED",
    "PARTIAL_FILL": "PARTIALLY_FILLED",
}

# Order side mappings
ORDER_SIDE_MAP = {
    "BUY": "BUY",
    "SELL": "SELL",
}

# Order type mappings (Rakuten Wallet -> NautilusTrader)
ORDER_TYPE_MAP = {
    "MARKET": "MARKET",
    "LIMIT": "LIMIT",
    "STOP": "STOP_MARKET",
}

# Reverse: NautilusTrader -> Rakuten Wallet
NAUTILUS_TO_RW_ORDER_TYPE = {
    "MARKET": "MARKET",
    "LIMIT": "LIMIT",
    "STOP_MARKET": "STOP",
}

# Candlestick type mappings (ISO 8601 durations)
# NautilusTrader BarSpecification (step, aggregation_name) -> Rakuten Wallet candlestickType
BAR_SPEC_TO_RW_INTERVAL = {
    (1, "MINUTE"): "PT1M",
    (5, "MINUTE"): "PT5M",
    (15, "MINUTE"): "PT15M",
    (30, "MINUTE"): "PT30M",
    (1, "HOUR"): "PT1H",
    (4, "HOUR"): "PT4H",
    (8, "HOUR"): "PT8H",
    (1, "DAY"): "P1D",
    (1, "WEEK"): "P1W",
    (1, "MONTH"): "P1M",
}

# Polling interval in seconds per bar interval (how often to fetch new bars)
BAR_POLL_INTERVALS = {
    "PT1M": 10,
    "PT5M": 30,
    "PT15M": 60,
    "PT30M": 120,
    "PT1H": 300,
    "PT4H": 600,
    "PT8H": 600,
    "P1D": 600,
    "P1W": 3600,
    "P1M": 3600,
}

# Error codes
ERROR_CODES = {
    10000: "Not found",
    10001: "System error",
    10005: "In maintenance",
    20001: "API key not found",
    20002: "Invalid API key",
    20003: "NONCE missing",
    20004: "Invalid NONCE",
    20005: "SIGNATURE missing",
    20006: "Invalid SIGNATURE",
    20010: "Too many requests",
    50003: "No orderbook available",
    50004: "Amount out of range",
    50005: "Price missing",
    50008: "Price out of range",
    50009: "Order not found",
    50020: "Amount outside min/max",
    50036: "Leverage outside min/max",
    50043: "Close amount exceeds position",
    50048: "Insufficient usable margin",
}
