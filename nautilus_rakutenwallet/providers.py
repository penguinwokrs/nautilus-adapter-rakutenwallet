# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2024 Penguinworks. All rights reserved.
#
#  Licensed under the GNU General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/gpl-3.0.en.html
# -------------------------------------------------------------------------------------------------
"""
Rakuten Wallet instrument provider implementation.
"""

from decimal import Decimal
import json
import logging

from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import CurrencyPair
from nautilus_trader.model.objects import Currency, Price, Quantity

RAKUTENW_VENUE = Venue("RAKUTENW")

logger = logging.getLogger(__name__)


class RakutenwInstrumentProvider(InstrumentProvider):
    """
    Provides Nautilus instrument definitions from Rakuten Wallet.

    Parameters
    ----------
    client : RakutenwRestClient
        The Rakuten Wallet REST client (Rust backend).
    config : InstrumentProviderConfig, optional
        The instrument provider configuration.
    """

    def __init__(
        self,
        client,
        config: InstrumentProviderConfig | None = None,
    ) -> None:
        super().__init__(config=config)
        self._client = client
        self._log_warnings = config.log_warnings if config else True

    async def load_all_async(self, filters: dict | None = None) -> None:
        filters_str = "..." if not filters else f" with filters {filters}..."
        self._log.info(f"Loading all instruments{filters_str}")

        try:
            symbols_json = await self._client.get_symbols_py()
            symbols_data = json.loads(symbols_json)

            if isinstance(symbols_data, dict):
                symbols_data = symbols_data.get("data", symbols_data)

            if not isinstance(symbols_data, list):
                symbols_data = []

            for symbol_info in symbols_data:
                try:
                    instrument = self._parse_instrument(symbol_info)
                    if instrument:
                        self.add(instrument=instrument)
                except Exception as e:
                    if self._log_warnings:
                        self._log.warning(f"Failed to parse instrument: {e}")

            self._log.info(f"Loaded {len(self._instruments)} instruments from Rakuten Wallet")

        except Exception as e:
            self._log.error(f"Failed to load instruments: {e}")
            raise

    async def load_ids_async(
        self,
        instrument_ids: list[InstrumentId],
        filters: dict | None = None,
    ) -> None:
        if not instrument_ids:
            self._log.warning("No instrument IDs given for loading")
            return

        for instrument_id in instrument_ids:
            if instrument_id.venue != RAKUTENW_VENUE:
                raise ValueError(
                    f"Instrument {instrument_id} is not for RAKUTENW venue"
                )

        await self.load_all_async(filters)

        requested_ids = set(instrument_ids)
        for instrument_id in list(self._instruments.keys()):
            if instrument_id not in requested_ids:
                self._instruments.pop(instrument_id, None)

    async def load_async(
        self,
        instrument_id: InstrumentId,
        filters: dict | None = None,
    ) -> None:
        await self.load_ids_async([instrument_id], filters)

    def _parse_instrument(self, symbol_info: dict) -> CurrencyPair | None:
        symbol_id = symbol_info.get("symbolId", "")
        if not symbol_id:
            return None

        symbol_name = symbol_info.get("symbolName", symbol_id)

        # Parse symbol: e.g., "BTC_JPY" -> base="BTC", quote="JPY"
        # or symbolId might be numeric, symbolName might be "BTC/JPY"
        if "_" in str(symbol_name):
            parts = symbol_name.split("_")
            base_asset = parts[0].upper()
            quote_asset = parts[1].upper() if len(parts) > 1 else "JPY"
        elif "/" in str(symbol_name):
            parts = symbol_name.split("/")
            base_asset = parts[0].upper()
            quote_asset = parts[1].upper() if len(parts) > 1 else "JPY"
        else:
            base_asset = symbol_name.upper()
            quote_asset = "JPY"

        # Parse precision from tickSize and tradeUnit
        tick_size_str = symbol_info.get("tickSize") or "1"
        trade_unit_str = symbol_info.get("tradeUnit") or "0.0001"

        tick_size = Decimal(tick_size_str)
        trade_unit = Decimal(trade_unit_str)

        price_precision = max(0, -tick_size.as_tuple().exponent)
        size_precision = max(0, -trade_unit.as_tuple().exponent)

        # Parse fees
        maker_fee = Decimal(symbol_info.get("makerFeeRate") or "0")
        taker_fee = Decimal(symbol_info.get("takerFeeRate") or "0")

        # Min/max amounts
        min_order_amount = symbol_info.get("minOrderAmount") or str(trade_unit)
        max_order_amount = symbol_info.get("maxOrderAmount")

        # Symbol string
        symbol_str = f"{base_asset}/{quote_asset}"

        instrument_id = InstrumentId(
            symbol=Symbol(symbol_str),
            venue=RAKUTENW_VENUE,
        )

        # Currencies
        from nautilus_trader.model.enums import CurrencyType

        base_currency = Currency(
            code=base_asset,
            precision=size_precision,
            iso4217=0,
            name=base_asset,
            currency_type=CurrencyType.CRYPTO,
        )
        quote_currency = Currency(
            code=quote_asset,
            precision=price_precision,
            iso4217=392 if quote_asset == "JPY" else 0,
            name=quote_asset,
            currency_type=CurrencyType.FIAT if quote_asset in ("JPY", "USD", "EUR") else CurrencyType.CRYPTO,
        )

        price_increment = Price(tick_size, precision=price_precision)
        size_increment = Quantity(trade_unit, precision=size_precision)

        # Leverage / margin info
        max_leverage = symbol_info.get("maxLeverage")
        margin_init = Decimal("1") / Decimal(max_leverage) if max_leverage else Decimal("0.5")
        margin_maint = margin_init  # Conservative default

        return CurrencyPair(
            instrument_id=instrument_id,
            raw_symbol=Symbol(str(symbol_id)),
            base_currency=base_currency,
            quote_currency=quote_currency,
            price_precision=price_precision,
            size_precision=size_precision,
            price_increment=price_increment,
            size_increment=size_increment,
            lot_size=Quantity(1, precision=0),
            max_quantity=Quantity.from_str(max_order_amount) if max_order_amount else None,
            min_quantity=Quantity.from_str(str(min_order_amount)),
            max_price=None,
            min_price=None,
            margin_init=margin_init,
            margin_maint=margin_maint,
            maker_fee=maker_fee,
            taker_fee=taker_fee,
            ts_event=0,
            ts_init=0,
        )
