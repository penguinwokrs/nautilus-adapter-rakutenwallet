import asyncio
import json
import logging
from datetime import datetime, timezone
from decimal import Decimal
from typing import Dict, List, Optional

from nautilus_trader.live.data_client import LiveMarketDataClient
from nautilus_trader.model.instruments import Instrument
from nautilus_trader.model.identifiers import ClientId, Venue
from .config import RakutenwDataClientConfig
from .constants import BAR_SPEC_TO_RW_INTERVAL, BAR_POLL_INTERVALS

try:
    from . import _nautilus_rakutenwallet as rakutenw
except ImportError:
    import _nautilus_rakutenwallet as rakutenw


class RakutenwDataClient(LiveMarketDataClient):
    """
    Rakuten Wallet live market data client.
    Actual low-level logic resides in Rust (RakutenwDataClient + RakutenwRestClient).
    """

    def __init__(self, loop, config: RakutenwDataClientConfig, msgbus, cache, clock, instrument_provider=None):
        if instrument_provider is None:
            from nautilus_trader.common.providers import InstrumentProvider
            instrument_provider = InstrumentProvider()

        super().__init__(
            loop=loop,
            client_id=ClientId("RAKUTENW-DATA"),
            venue=Venue("RAKUTENW"),
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
            config=config,
        )
        self.config = config
        self._logger = logging.getLogger(__name__)
        self._subscribed_instruments = {}  # symbol_id -> Instrument
        self._symbol_id_map = {}  # "BTC/JPY" -> symbol_id (raw_symbol)
        self._bar_poll_tasks: Dict[str, asyncio.Task] = {}
        self._bar_last_timestamps: Dict[str, str] = {}

        # Rust clients
        self._rust_client = rakutenw.RakutenwDataClient(
            getattr(self.config, 'ws_rate_limit_per_sec', None),
        )
        self._rust_client.set_data_callback(self._handle_rust_data)

        self._rest_client = rakutenw.RakutenwRestClient(
            self.config.api_key or "",
            self.config.api_secret or "",
            self.config.timeout_ms,
            self.config.proxy_url,
            getattr(self.config, 'rate_limit_per_sec', None),
        )

    async def _connect(self):
        self._logger.info("RakutenwDataClient connecting")

        # Load instruments
        await self._load_instruments()

        # Connect Rust DataClient (Public WebSocket)
        await self._rust_client.connect()
        self._logger.info("Connected to Rakuten Wallet via Rust client (Public WebSocket)")

    async def _disconnect(self):
        for task in self._bar_poll_tasks.values():
            if not task.done():
                task.cancel()
        self._bar_poll_tasks.clear()
        self._bar_last_timestamps.clear()
        await self._rust_client.disconnect()

    async def subscribe(self, instruments: List[Instrument]):
        for instrument in instruments:
            # Get the raw symbol (symbol_id for Rakuten Wallet API)
            symbol_id = str(instrument.raw_symbol)
            self._subscribed_instruments[symbol_id] = instrument
            self._symbol_id_map[instrument.id.symbol.value] = symbol_id

            # Subscribe to all channels for this symbol
            await self._rust_client.subscribe("TICKER", symbol_id)
            await self._rust_client.subscribe("TRADES", symbol_id)
            await self._rust_client.subscribe("ORDERBOOK", symbol_id)

        self._logger.info(f"Subscribed to {len(instruments)} instruments")

    async def unsubscribe(self, instruments: List[Instrument]):
        pass

    def _handle_rust_data(self, *args):
        """
        Callback from Rust. May receive 2 or 3 args:
        - (channel, data) for TICKER and ORDERBOOK
        - (channel, data, symbol_id) for TRADES
        """
        try:
            channel = args[0]
            data = args[1]
            if channel == "TICKER":
                self._handle_ticker(data)
            elif channel == "ORDERBOOK":
                self._handle_orderbook(data)
            elif channel == "TRADES":
                symbol_id = args[2] if len(args) > 2 else None
                self._handle_trade(data, symbol_id)
        except Exception as e:
            self._logger.error(f"Error handling data from Rust: {e}")

    def _handle_ticker(self, data):
        symbol_id = data.symbol_id
        instrument = self._subscribed_instruments.get(symbol_id)
        if not instrument:
            return

        from nautilus_trader.model.data import QuoteTick
        from nautilus_trader.model.objects import Price, Quantity

        bid = data.best_bid
        ask = data.best_ask

        if bid and ask:
            quote = QuoteTick(
                instrument_id=instrument.id,
                bid_price=Price.from_str(str(bid)),
                ask_price=Price.from_str(str(ask)),
                bid_size=Quantity.from_str("0"),
                ask_size=Quantity.from_str("0"),
                ts_event=self._clock.timestamp_ns(),
                ts_init=self._clock.timestamp_ns(),
            )
            self._handle_data(quote)

    def _handle_trade(self, data, symbol_id=None):
        if symbol_id is None:
            return
        instrument = self._subscribed_instruments.get(symbol_id)
        if not instrument:
            return

        from nautilus_trader.model.data import TradeTick
        from nautilus_trader.model.objects import Price, Quantity
        from nautilus_trader.model.enums import AggressorSide
        from nautilus_trader.model.identifiers import TradeId

        side_str = data.order_side
        aggressor_side = AggressorSide.BUYER if side_str == "BUY" else AggressorSide.SELLER

        tick = TradeTick(
            instrument_id=instrument.id,
            price=Price.from_str(str(data.price)),
            size=Quantity.from_str(str(data.amount)),
            aggressor_side=aggressor_side,
            trade_id=TradeId(str(data.id)),
            ts_event=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )
        self._handle_data(tick)

    def _handle_orderbook(self, data):
        symbol_id = data.symbol_id
        instrument = self._subscribed_instruments.get(symbol_id)
        if not instrument:
            return

        from nautilus_trader.model.data import OrderBookDelta, OrderBookDeltas, BookOrder
        from nautilus_trader.model.enums import BookAction, OrderSide
        from nautilus_trader.model.objects import Price, Quantity

        top_asks, top_bids = data.get_top_n(self.config.order_book_depth)
        ts_init = self._clock.timestamp_ns()

        deltas = []
        deltas.append(OrderBookDelta.clear(instrument.id, 0, ts_init, ts_init))

        for p, q in top_asks:
            order = BookOrder(OrderSide.SELL, Price.from_str(str(p)), Quantity.from_str(str(q)), 0)
            deltas.append(OrderBookDelta(instrument.id, BookAction.ADD, order, 0, 0, ts_init, ts_init))

        for p, q in top_bids:
            order = BookOrder(OrderSide.BUY, Price.from_str(str(p)), Quantity.from_str(str(q)), 0)
            deltas.append(OrderBookDelta(instrument.id, BookAction.ADD, order, 0, 0, ts_init, ts_init))

        snapshot = OrderBookDeltas(instrument.id, deltas)
        self._handle_data(snapshot)

    async def fetch_instruments(self) -> List[Instrument]:
        from nautilus_trader.model.instruments import CurrencyPair
        from nautilus_trader.model.identifiers import InstrumentId, Symbol
        from nautilus_trader.model.objects import Price, Quantity, Currency
        from nautilus_trader.model.enums import CurrencyType
        import nautilus_trader.model.currencies as currencies

        def get_currency(code: str) -> Currency:
            code = code.upper()
            if hasattr(currencies, code):
                return getattr(currencies, code)
            return Currency(code, 8, 0, code, CurrencyType.CRYPTO)

        try:
            res_json = await self._rest_client.get_symbols_py()
            data = json.loads(res_json)

            if isinstance(data, dict):
                symbols = data.get("data", data)
            else:
                symbols = data

            instruments = []
            for s in symbols:
                symbol_id = s.get("symbolId", "")
                symbol_name = s.get("symbolName", symbol_id)

                if "_" in str(symbol_name):
                    parts = symbol_name.split("_")
                    base = parts[0].upper()
                    quote = parts[1].upper() if len(parts) > 1 else "JPY"
                elif "/" in str(symbol_name):
                    parts = symbol_name.split("/")
                    base = parts[0].upper()
                    quote = parts[1].upper() if len(parts) > 1 else "JPY"
                else:
                    base = symbol_name.upper()
                    quote = "JPY"

                symbol_str = f"{base}/{quote}"
                tick_size = Decimal(s.get("tickSize", "1"))
                trade_unit = Decimal(s.get("tradeUnit", "0.0001"))
                p_prec = max(0, -tick_size.as_tuple().exponent)
                q_prec = max(0, -trade_unit.as_tuple().exponent)
                min_q = s.get("minOrderAmount", str(trade_unit))

                instrument = CurrencyPair(
                    instrument_id=InstrumentId.from_str(f"{symbol_str}.RAKUTENW"),
                    raw_symbol=Symbol(str(symbol_id)),
                    base_currency=get_currency(base),
                    quote_currency=get_currency(quote),
                    price_precision=p_prec,
                    size_precision=q_prec,
                    price_increment=Price(tick_size, p_prec),
                    size_increment=Quantity(trade_unit, q_prec),
                    min_quantity=Quantity.from_str(str(min_q)),
                    ts_event=0,
                    ts_init=0,
                )
                instruments.append(instrument)

            self._logger.info(f"Fetched {len(instruments)} instruments from Rakuten Wallet")
            return instruments

        except Exception as e:
            self._logger.error(f"Error fetching instruments: {e}")
            return []

    async def _subscribe_quote_ticks(self, command):
        instrument_id = command.instrument_id if hasattr(command, 'instrument_id') else command
        instrument = self._instrument_provider.find(instrument_id)
        if instrument is None and hasattr(self, '_cache'):
            instrument = self._cache.instrument(instrument_id)

        if instrument:
            await self.subscribe([instrument])
        else:
            self._logger.error(f"Could not find instrument {instrument_id}")

    async def _unsubscribe_quote_ticks(self, instrument_id):
        pass

    async def _subscribe_trade_ticks(self, command):
        instrument_id = command.instrument_id if hasattr(command, 'instrument_id') else command
        instrument = self._instrument_provider.find(instrument_id)
        if instrument is None and hasattr(self, '_cache'):
            instrument = self._cache.instrument(instrument_id)

        if instrument:
            await self.subscribe([instrument])
        else:
            self._logger.error(f"Could not find instrument {instrument_id}")

    async def _unsubscribe_trade_ticks(self, instrument_id):
        pass

    async def _subscribe_order_book_deltas(self, command):
        instrument_id = command.instrument_id if hasattr(command, 'instrument_id') else command
        instrument = self._instrument_provider.find(instrument_id)
        if instrument is None and hasattr(self, '_cache'):
            instrument = self._cache.instrument(instrument_id)

        if instrument:
            await self.subscribe([instrument])
        else:
            self._logger.error(f"Could not find instrument {instrument_id}")

    async def _unsubscribe_order_book_deltas(self, instrument_id):
        pass

    async def _subscribe_order_book_snapshots(self, instrument_id):
        pass

    async def _unsubscribe_order_book_snapshots(self, instrument_id):
        pass

    async def _subscribe_bars(self, command):
        from nautilus_trader.model.data import BarType
        from nautilus_trader.model.enums import BarAggregation

        bar_type = command.bar_type if hasattr(command, 'bar_type') else command
        spec = bar_type.spec
        step = spec.step
        agg_name = BarAggregation(spec.aggregation).name

        rw_interval = BAR_SPEC_TO_RW_INTERVAL.get((step, agg_name))
        if rw_interval is None:
            self._logger.warning(
                f"Unsupported bar specification: {step}-{agg_name}. "
                f"Supported: {list(BAR_SPEC_TO_RW_INTERVAL.keys())}"
            )
            return

        bar_type_str = str(bar_type)
        if bar_type_str in self._bar_poll_tasks:
            self._logger.info(f"Already subscribed to bars: {bar_type_str}")
            return

        instrument_id = bar_type.instrument_id
        symbol_id = self._symbol_id_map.get(instrument_id.symbol.value, instrument_id.symbol.value)
        poll_interval = BAR_POLL_INTERVALS.get(rw_interval, 60)

        self._logger.info(
            f"Subscribing to bars: {bar_type_str} (RW interval={rw_interval}, poll={poll_interval}s)"
        )

        task = self.create_task(
            self._bar_poll_loop(bar_type, symbol_id, rw_interval, poll_interval)
        )
        self._bar_poll_tasks[bar_type_str] = task

    async def _unsubscribe_bars(self, command):
        from nautilus_trader.model.data import BarType

        bar_type = command.bar_type if hasattr(command, 'bar_type') else command
        bar_type_str = str(bar_type)

        task = self._bar_poll_tasks.pop(bar_type_str, None)
        if task and not task.done():
            task.cancel()
            self._logger.info(f"Unsubscribed from bars: {bar_type_str}")

    async def _bar_poll_loop(self, bar_type, symbol_id: str, rw_interval: str, poll_interval: int):
        from nautilus_trader.model.data import Bar
        from nautilus_trader.model.objects import Price, Quantity

        bar_type_str = str(bar_type)

        try:
            while True:
                try:
                    date_from = datetime.now(timezone.utc).strftime("%Y-%m-%dT00:00:00.000Z")
                    resp_json = await self._rest_client.get_candlestick_py(symbol_id, rw_interval, date_from)
                    klines = json.loads(resp_json)

                    if isinstance(klines, dict):
                        klines = klines.get("data", klines)
                    if not isinstance(klines, list):
                        klines = []

                    last_ts = self._bar_last_timestamps.get(bar_type_str)

                    for kline in klines:
                        open_time = str(kline.get("openTime", ""))
                        if not open_time:
                            continue

                        if last_ts and open_time <= last_ts:
                            continue

                        ts_event = int(open_time) * 1_000_000 if open_time.isdigit() else self._clock.timestamp_ns()
                        ts_init = self._clock.timestamp_ns()

                        bar = Bar(
                            bar_type=bar_type,
                            open=Price.from_str(str(kline["open"])),
                            high=Price.from_str(str(kline["high"])),
                            low=Price.from_str(str(kline["low"])),
                            close=Price.from_str(str(kline["close"])),
                            volume=Quantity.from_str(str(kline["volume"])),
                            ts_event=ts_event,
                            ts_init=ts_init,
                        )
                        self._handle_data(bar)
                        self._bar_last_timestamps[bar_type_str] = open_time

                except asyncio.CancelledError:
                    raise
                except Exception as e:
                    self._logger.error(f"Error polling bars for {bar_type_str}: {e}")

                await asyncio.sleep(poll_interval)

        except asyncio.CancelledError:
            self._logger.info(f"Bar polling stopped for {bar_type_str}")
        except Exception as e:
            self._logger.error(f"Bar poll loop crashed for {bar_type_str}: {e}")

    async def _load_instruments(self):
        if not self.config.instrument_provider or not self.config.instrument_provider.load_ids:
            return

        try:
            from nautilus_trader.model.instruments import CurrencyPair
            from nautilus_trader.model.identifiers import Symbol, Venue, InstrumentId
            from nautilus_trader.model.objects import Price, Quantity
            from nautilus_trader.model.currencies import Currency
        except ImportError as e:
            self._logger.error(f"Imports failed: {e}")
            return

        def add_instrument(symbol_str: str, symbol_id: str, base: str, quote: str, p_prec: int, q_prec: int, min_q: str, tick_size, trade_unit):
            try:
                instrument_id = InstrumentId.from_str(f"{symbol_str}.RAKUTENW")

                exists_in_provider = False
                try:
                    if self._instrument_provider.find(instrument_id):
                        exists_in_provider = True
                except Exception:
                    pass

                instrument = CurrencyPair(
                    instrument_id=instrument_id,
                    raw_symbol=Symbol(str(symbol_id)),
                    base_currency=Currency.from_str(base),
                    quote_currency=Currency.from_str(quote),
                    price_precision=p_prec,
                    size_precision=q_prec,
                    price_increment=Price(tick_size, p_prec),
                    size_increment=Quantity(trade_unit, q_prec),
                    ts_event=0,
                    ts_init=0,
                    min_quantity=Quantity(Decimal(min_q), q_prec),
                    lot_size=None,
                )

                if not exists_in_provider:
                    self._instrument_provider.add(instrument)
                    self._logger.info(f"Loaded instrument {instrument_id} to provider")

                if self._cache:
                    self._cache.add_instrument(instrument)

                # Store symbol mapping
                self._symbol_id_map[symbol_str] = symbol_id

            except Exception as e:
                self._logger.error(f"Failed to add instrument {symbol_str}: {e}")

        # Fetch from Rakuten Wallet API
        try:
            res_json = await self._rest_client.get_symbols_py()
            data = json.loads(res_json)
            if isinstance(data, dict):
                symbols = data.get("data", [])
            elif isinstance(data, list):
                symbols = data
            else:
                symbols = []

            symbols_map = {}
            for s in symbols:
                sid = s.get("symbolId", "")
                name = s.get("symbolName", sid)
                if "_" in str(name):
                    parts = name.split("_")
                    key = f"{parts[0].upper()}/{parts[1].upper()}" if len(parts) > 1 else parts[0].upper()
                elif "/" in str(name):
                    key = name.upper()
                else:
                    key = f"{name.upper()}/JPY"
                symbols_map[key] = (s, str(sid))

            for instrument_id_str in self.config.instrument_provider.load_ids:
                try:
                    if "." in instrument_id_str:
                        native_symbol = instrument_id_str.split(".")[0]
                    else:
                        native_symbol = instrument_id_str

                    base = native_symbol.split("/")[0].upper()
                    quote = native_symbol.split("/")[1].upper() if "/" in native_symbol else "JPY"
                    lookup_key = f"{base}/{quote}"

                    info_tuple = symbols_map.get(lookup_key)
                    if info_tuple:
                        info, symbol_id = info_tuple
                        tick_size = Decimal(info.get("tickSize", "1"))
                        trade_unit = Decimal(info.get("tradeUnit", "0.0001"))
                        p_prec = max(0, -tick_size.as_tuple().exponent)
                        q_prec = max(0, -trade_unit.as_tuple().exponent)
                        min_q = info.get("minOrderAmount", str(trade_unit))
                        add_instrument(lookup_key, symbol_id, base, quote, p_prec, q_prec, min_q, tick_size, trade_unit)
                    else:
                        self._logger.warning(f"Symbol {lookup_key} not found in Rakuten Wallet API")
                except Exception as e:
                    self._logger.error(f"Error processing instrument {instrument_id_str}: {e}")

        except Exception as e:
            self._logger.warning(f"Error fetching symbols API: {e}")
