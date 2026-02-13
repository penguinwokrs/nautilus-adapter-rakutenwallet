import asyncio
import json
import logging
from typing import Dict, List, Optional
from decimal import Decimal

from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.model.orders import Order
from nautilus_trader.model.objects import Money, Currency, AccountBalance
from nautilus_trader.model.events import AccountState
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.model.currencies import JPY
from nautilus_trader.model.identifiers import Venue, ClientId, AccountId, VenueOrderId
from nautilus_trader.model.enums import (
    OrderSide, OrderType, OmsType, AccountType, OrderStatus,
    TimeInForce, LiquiditySide,
)
from nautilus_trader.execution.messages import (
    SubmitOrder, CancelOrder, ModifyOrder,
    GenerateOrderStatusReports, GenerateFillReports, GeneratePositionStatusReports,
)
from nautilus_trader.execution.reports import OrderStatusReport, FillReport, PositionStatusReport
from nautilus_trader.model.identifiers import TradeId, PositionId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.enums import PositionSide

from .config import RakutenwExecClientConfig
from .constants import NAUTILUS_TO_RW_ORDER_TYPE, ORDER_STATUS_MAP, ORDER_TYPE_MAP

try:
    from . import _nautilus_rakutenwallet as rakutenw
except ImportError:
    import _nautilus_rakutenwallet as rakutenw


class RakutenwExecutionClient(LiveExecutionClient):
    """
    Rakuten Wallet live execution client.
    Wraps Rust RakutenwExecutionClient for REST orders and polling-based updates.
    """

    def __init__(self, loop, config: RakutenwExecClientConfig, msgbus, cache, clock, instrument_provider: InstrumentProvider):
        super().__init__(
            loop=loop,
            client_id=ClientId("RAKUTENW"),
            venue=Venue("RAKUTENW"),
            oms_type=OmsType.HEDGING,
            account_type=AccountType.MARGIN,
            base_currency=None,  # Multi-currency
            instrument_provider=instrument_provider,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            config=config,
        )
        self.config = config
        self._instrument_provider = instrument_provider
        self._logger = logging.getLogger(__name__)
        self._account_id = AccountId("RAKUTENW-001")
        self._set_account_id(self._account_id)
        self._order_states = {}

        poll_ms = int(config.order_poll_interval_sec * 1000)
        self._rust_client = rakutenw.RakutenwExecutionClient(
            self.config.api_key or "",
            self.config.api_secret or "",
            self.config.timeout_ms,
            self.config.proxy_url,
            getattr(self.config, 'rate_limit_per_sec', None),
            poll_ms,
        )
        self._rust_client.set_order_callback(self._handle_poll_message)

        self._rest_client = rakutenw.RakutenwRestClient(
            self.config.api_key or "",
            self.config.api_secret or "",
            self.config.timeout_ms,
            self.config.proxy_url,
            getattr(self.config, 'rate_limit_per_sec', None),
        )
        self.log = logging.getLogger("nautilus.rakutenw.execution")

    @property
    def account_id(self) -> AccountId:
        return self._account_id

    async def _connect(self):
        # Register all currencies
        await self._register_all_currencies()

        try:
            # Connect Rust client (order polling loop)
            await self._rust_client.connect()
            self.log.info("Order polling loop started via Rust client")

            # Initial account state
            try:
                reports = await self.generate_account_status_reports()
                if reports:
                    for report in reports:
                        self._send_account_state(report)
                    self.log.info(f"Published {len(reports)} account reports")
            except Exception as e:
                self.log.error(f"Failed to fetch initial account state: {e}")

        except Exception as e:
            self.log.error(f"Failed to connect: {e}")

    async def _disconnect(self):
        self.log.info("RakutenwExecutionClient disconnected")

    def submit_order(self, command: SubmitOrder) -> None:
        self.create_task(self._submit_order(command))

    async def _submit_order(self, command: SubmitOrder) -> None:
        try:
            order = command.order
            instrument_id = order.instrument_id
            instrument = self._instrument_provider.find(instrument_id)
            if instrument is None and hasattr(self, '_cache'):
                instrument = self._cache.instrument(instrument_id)

            # Get symbol_id from raw_symbol
            symbol_id = str(instrument.raw_symbol) if instrument else instrument_id.symbol.value

            side = "BUY" if order.side == OrderSide.BUY else "SELL"

            order_type = "MARKET"
            price = None
            if order.order_type == OrderType.LIMIT:
                order_type = "LIMIT"
                price = str(order.price)
            elif order.order_type == OrderType.STOP_MARKET:
                order_type = "STOP"
                price = str(order.trigger_price) if hasattr(order, 'trigger_price') else None
            elif order.order_type != OrderType.MARKET:
                return  # Unsupported

            # Default to OPEN behavior
            order_behavior = "OPEN"
            leverage = None
            close_behavior = None
            post_only = None

            # Check for post_only
            if hasattr(order, 'is_post_only') and order.is_post_only:
                post_only = True

            # Extract tags for Rakuten Wallet specific parameters
            if hasattr(order, 'tags') and order.tags:
                for tag in order.tags:
                    tag_str = str(tag)
                    if tag_str.startswith("orderBehavior="):
                        order_behavior = tag_str.split("=", 1)[1]
                    elif tag_str.startswith("leverage="):
                        leverage = tag_str.split("=", 1)[1]
                    elif tag_str.startswith("closeBehavior="):
                        close_behavior = tag_str.split("=", 1)[1]

            amount = str(order.quantity)
            client_id = str(order.client_order_id)

            resp_json = await self._rust_client.submit_order(
                symbol_id, order_behavior, side, order_type, amount, client_id,
                price, leverage, close_behavior, post_only,
            )

            resp = json.loads(resp_json)
            venue_order_id = VenueOrderId(str(resp.get("order_id")))

            self.generate_order_accepted(
                strategy_id=order.strategy_id,
                instrument_id=order.instrument_id,
                client_order_id=order.client_order_id,
                venue_order_id=venue_order_id,
                ts_event=self._clock.timestamp_ns(),
            )

        except Exception as e:
            self._logger.error(f"Submit failed: {e}")

    def cancel_order(self, command: CancelOrder) -> None:
        self.create_task(self._cancel_order(command))

    async def _cancel_order(self, command: CancelOrder) -> None:
        try:
            if not command.venue_order_id:
                return

            instrument_id = command.instrument_id
            instrument = self._instrument_provider.find(instrument_id)
            if instrument is None and hasattr(self, '_cache'):
                instrument = self._cache.instrument(instrument_id)

            symbol_id = str(instrument.raw_symbol) if instrument else instrument_id.symbol.value

            await self._rust_client.cancel_order(
                symbol_id,
                str(command.venue_order_id),
            )

            self.generate_order_canceled(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=command.venue_order_id,
                ts_event=self._clock.timestamp_ns(),
            )

        except Exception as e:
            self._logger.error(f"Cancel failed: {e}")

    def modify_order(self, command: ModifyOrder) -> None:
        self.create_task(self._modify_order(command))

    async def _modify_order(self, command: ModifyOrder) -> None:
        try:
            if not command.venue_order_id:
                self._logger.error("ModifyOrder requires venue_order_id")
                return

            venue_order_id_str = str(command.venue_order_id)
            new_price = str(command.price) if command.price else None
            new_qty = str(command.quantity) if command.quantity else None

            await self._rust_client.modify_order(
                venue_order_id_str,
                new_price,
                new_qty,
            )

            self.generate_order_updated(
                strategy_id=command.strategy_id,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=command.venue_order_id,
                quantity=command.quantity if command.quantity else None,
                price=command.price,
                trigger_price=command.trigger_price,
                ts_event=self._clock.timestamp_ns(),
            )

        except Exception as e:
            self._logger.error(f"Modify failed: {e}")

    def _handle_poll_message(self, event_type: str, message: str):
        """Handle incoming order poll message from Rust client."""
        self.log.debug(f"Poll Event Received: {event_type}")
        try:
            data = json.loads(message)
            if event_type == "OrderUpdate":
                order_id = data.get("id") or data.get("orderId")
                if order_id:
                    venue_order_id = VenueOrderId(str(order_id))
                    self.create_task(self._process_order_update_from_data(venue_order_id, data))
            elif event_type == "OrderCompleted":
                order_id = data.get("orderId")
                if order_id:
                    venue_order_id = VenueOrderId(str(order_id))
                    self.create_task(self._process_order_completed(venue_order_id))
            else:
                self.log.debug(f"Unknown poll event: {event_type}")
        except Exception as e:
            self.log.error(f"Error handling poll message: {e}")

    async def _process_order_completed(self, venue_order_id: VenueOrderId):
        """Handle an order that is no longer in the active orders list."""
        client_oid = None
        for _ in range(10):
            client_oid = self._cache.client_order_id(venue_order_id)
            if client_oid:
                break
            await asyncio.sleep(0.1)

        if not client_oid:
            self._logger.warning(
                f"ClientOrderId not found for venue_order_id: {venue_order_id}"
            )
            return

        order = self._cache.order(client_oid)
        if not order:
            return

        # Try to get trade details to determine if filled or canceled
        try:
            trades_json = await self._rust_client.get_trades_for_order(str(venue_order_id))
            trades_data = json.loads(trades_json)
            trades_list = trades_data if isinstance(trades_data, list) else trades_data.get("list", trades_data.get("data", []))

            if trades_list:
                # Order was filled - process fills
                instrument = self._instrument_provider.find(order.instrument_id)
                if instrument is None and hasattr(self, '_cache'):
                    instrument = self._cache.instrument(order.instrument_id)
                quote_currency = JPY if not instrument else instrument.quote_currency

                oid_str = str(venue_order_id)
                if oid_str not in self._order_states:
                    self._order_states[oid_str] = {"reported_trades": set()}

                state = self._order_states[oid_str]

                for trade in trades_list:
                    trade_id = str(trade.get("id", ""))
                    if trade_id in state["reported_trades"]:
                        continue
                    state["reported_trades"].add(trade_id)

                    qty = Decimal(trade.get("amount", "0"))
                    px = Decimal(trade.get("price", "0"))
                    fee = Decimal(trade.get("fee", "0") or "0")

                    self.generate_order_filled(
                        strategy_id=order.strategy_id,
                        instrument_id=order.instrument_id,
                        client_order_id=order.client_order_id,
                        venue_order_id=venue_order_id,
                        venue_position_id=None,
                        fill_id=None,
                        last_qty=qty,
                        last_px=px,
                        liquidity=None,
                        commission=Money(fee, quote_currency),
                        ts_event=self._clock.timestamp_ns(),
                    )
            else:
                # No trades found - order was canceled
                if order.status not in (OrderStatus.CANCELED, OrderStatus.FILLED, OrderStatus.EXPIRED):
                    self.generate_order_canceled(
                        strategy_id=order.strategy_id,
                        instrument_id=order.instrument_id,
                        client_order_id=order.client_order_id,
                        venue_order_id=venue_order_id,
                        ts_event=self._clock.timestamp_ns(),
                    )
        except Exception as e:
            self._logger.error(f"Error processing order completion: {e}")

    async def _process_order_update_from_data(self, venue_order_id: VenueOrderId, data: dict):
        client_oid = None
        for _ in range(10):
            client_oid = self._cache.client_order_id(venue_order_id)
            if client_oid:
                break
            await asyncio.sleep(0.1)

        if not client_oid:
            self._logger.warning(
                f"ClientOrderId not found for venue_order_id: {venue_order_id} after retries."
            )
            return

        order = self._cache.order(client_oid)
        if not order:
            self._logger.warning(f"Order not found in cache for client_order_id: {client_oid}")
            return

        instrument = self._instrument_provider.find(order.instrument_id)
        if instrument is None and hasattr(self, '_cache'):
            instrument = self._cache.instrument(order.instrument_id)

        quote_currency = JPY if not instrument else instrument.quote_currency

        # Check for partial fills
        executed_amount = Decimal(data.get("executedAmount", "0") or "0")
        oid_str = str(venue_order_id)
        if oid_str not in self._order_states:
            self._order_states[oid_str] = {
                "last_executed_qty": Decimal("0"),
                "reported_trades": set(),
            }

        state = self._order_states[oid_str]
        last_qty = state.get("last_executed_qty", Decimal("0"))

        if executed_amount > last_qty:
            delta = executed_amount - last_qty
            avg_price = Decimal(data.get("price", "0") or "0")
            commission = Money(Decimal("0"), quote_currency)

            self.generate_order_filled(
                strategy_id=order.strategy_id,
                instrument_id=order.instrument_id,
                client_order_id=order.client_order_id,
                venue_order_id=venue_order_id,
                venue_position_id=None,
                fill_id=None,
                last_qty=delta,
                last_px=avg_price,
                liquidity=None,
                commission=commission,
                ts_event=self._clock.timestamp_ns(),
            )

            state["last_executed_qty"] = executed_amount

    # Required abstract methods

    async def generate_order_status_reports(self, command: GenerateOrderStatusReports) -> list[OrderStatusReport]:
        reports = []
        try:
            instrument_id = command.instrument_id

            RW_ORDER_STATUS_MAP = {
                "WORKING_ORDER": OrderStatus.ACCEPTED,
                "PARTIAL_FILL": OrderStatus.PARTIALLY_FILLED,
            }
            RW_ORDER_TYPE_MAP = {
                "MARKET": OrderType.MARKET,
                "LIMIT": OrderType.LIMIT,
                "STOP": OrderType.STOP_MARKET,
            }

            resp_json = await self._rust_client.get_orders(
                symbol_id=None,
                order_status=["WORKING_ORDER", "PARTIAL_FILL"],
            )
            resp = json.loads(resp_json)
            orders_list = resp if isinstance(resp, list) else resp.get("list", resp.get("data", []))

            for order_data in orders_list:
                try:
                    venue_oid = VenueOrderId(str(order_data.get("id")))
                    side_str = order_data.get("orderSide", "BUY")
                    order_side = OrderSide.BUY if side_str == "BUY" else OrderSide.SELL

                    ot_str = order_data.get("orderType", "LIMIT")
                    order_type = RW_ORDER_TYPE_MAP.get(ot_str, OrderType.LIMIT)

                    status_str = order_data.get("orderStatus", "WORKING_ORDER")
                    order_status = RW_ORDER_STATUS_MAP.get(status_str, OrderStatus.ACCEPTED)

                    amount = Decimal(order_data.get("amount", "0"))
                    executed_amount = Decimal(order_data.get("executedAmount", "0") or "0")

                    price_str = order_data.get("price")
                    price = Price(Decimal(price_str), precision=0) if price_str else None

                    # Build instrument_id from symbolId
                    symbol_id = order_data.get("symbolId", "")
                    from nautilus_trader.model.identifiers import InstrumentId, Symbol
                    # Try to find instrument by raw_symbol
                    inst_id = instrument_id
                    if not inst_id:
                        for iid, inst in self._instrument_provider.get_all().items() if hasattr(self._instrument_provider.get_all(), 'items') else []:
                            if str(inst.raw_symbol) == str(symbol_id):
                                inst_id = iid
                                break
                        if not inst_id:
                            inst_id = InstrumentId(Symbol(f"{symbol_id}"), Venue("RAKUTENW"))

                    ts_now = self._clock.timestamp_ns()

                    report = OrderStatusReport(
                        account_id=self._account_id,
                        instrument_id=inst_id,
                        venue_order_id=venue_oid,
                        order_side=order_side,
                        order_type=order_type,
                        time_in_force=TimeInForce.GTC,
                        order_status=order_status,
                        quantity=Quantity(amount, precision=8),
                        filled_qty=Quantity(executed_amount, precision=8),
                        report_id=UUID4(),
                        ts_accepted=ts_now,
                        ts_last=ts_now,
                        ts_init=ts_now,
                        price=price,
                    )
                    reports.append(report)
                except Exception as e:
                    self._logger.warning(f"Failed to parse order report: {e}")
                    continue

        except Exception as e:
            self._logger.error(f"Failed to generate order status reports: {e}")

        return reports

    async def generate_account_status_reports(self, instrument_id=None, client_order_id=None):
        try:
            reports = []
            # Get equity data for margin account
            equity_json = await self._rust_client.get_equity_data_py()
            self.log.debug(f"Fetched equity data: {equity_json[:200]}...")

            equity_data = json.loads(equity_json)

            usable_amount = Decimal(equity_data.get("usableAmount", "0") or "0")
            required_margin = Decimal(equity_data.get("requiredMarginAmount", "0") or "0")
            order_margin = Decimal(equity_data.get("orderMarginAmount", "0") or "0")

            total = usable_amount + required_margin + order_margin
            locked = required_margin + order_margin

            balance = AccountBalance(
                Money(total, JPY),
                Money(locked, JPY),
                Money(usable_amount, JPY),
            )

            account_state = AccountState(
                self._account_id,
                self.account_type,
                None,
                True,
                [balance],
                [],
                {},
                UUID4(),
                self._clock.timestamp_ns(),
                self._clock.timestamp_ns(),
            )
            reports.append(account_state)
            return reports

        except Exception as e:
            self._logger.error(f"Failed to generate account status reports: {e}", exc_info=True)
            return []

    async def generate_fill_reports(self, command: GenerateFillReports) -> list[FillReport]:
        reports = []
        try:
            resp_json = await self._rust_client.get_trades_for_order(
                str(command.venue_order_id) if command.venue_order_id else ""
            )
            resp = json.loads(resp_json)
            exec_list = resp if isinstance(resp, list) else resp.get("list", resp.get("data", []))

            for exec_data in exec_list:
                try:
                    venue_oid = VenueOrderId(str(exec_data.get("orderId", "")))
                    trade_id = TradeId(str(exec_data.get("id")))
                    side_str = exec_data.get("orderSide", "BUY")
                    order_side = OrderSide.BUY if side_str == "BUY" else OrderSide.SELL

                    exec_size = Decimal(exec_data.get("amount", "0"))
                    exec_price = Decimal(exec_data.get("price", "0"))
                    fee = Decimal(exec_data.get("fee", "0") or "0")

                    inst_id = command.instrument_id
                    if not inst_id:
                        from nautilus_trader.model.identifiers import InstrumentId, Symbol
                        symbol_id = exec_data.get("symbolId", "")
                        inst_id = InstrumentId(Symbol(str(symbol_id)), Venue("RAKUTENW"))

                    ts_now = self._clock.timestamp_ns()

                    report = FillReport(
                        account_id=self._account_id,
                        instrument_id=inst_id,
                        venue_order_id=venue_oid,
                        trade_id=trade_id,
                        order_side=order_side,
                        last_qty=Quantity(exec_size, precision=8),
                        last_px=Price(exec_price, precision=0),
                        commission=Money(fee, JPY),
                        liquidity_side=LiquiditySide.NO_LIQUIDITY_SIDE,
                        report_id=UUID4(),
                        ts_event=ts_now,
                        ts_init=ts_now,
                    )
                    reports.append(report)
                except Exception as e:
                    self._logger.warning(f"Failed to parse fill report: {e}")
                    continue

        except Exception as e:
            self._logger.error(f"Failed to generate fill reports: {e}")

        return reports

    async def generate_position_status_reports(self, command: GeneratePositionStatusReports) -> list[PositionStatusReport]:
        reports = []
        try:
            resp_json = await self._rust_client.get_positions(symbol_id=None)
            resp = json.loads(resp_json)
            pos_list = resp if isinstance(resp, list) else resp.get("list", resp.get("data", []))

            for pos_data in pos_list:
                try:
                    position_id = PositionId(str(pos_data.get("id")))
                    side_str = pos_data.get("orderSide", "BUY")
                    position_side = PositionSide.LONG if side_str == "BUY" else PositionSide.SHORT

                    size = Decimal(pos_data.get("amount", "0"))

                    symbol_id = pos_data.get("symbolId", "")
                    inst_id = command.instrument_id
                    if not inst_id:
                        from nautilus_trader.model.identifiers import InstrumentId, Symbol
                        inst_id = InstrumentId(Symbol(str(symbol_id)), Venue("RAKUTENW"))

                    ts_now = self._clock.timestamp_ns()

                    report = PositionStatusReport(
                        account_id=self._account_id,
                        instrument_id=inst_id,
                        position_side=position_side,
                        quantity=Quantity(size, precision=8),
                        report_id=UUID4(),
                        ts_last=ts_now,
                        ts_init=ts_now,
                    )
                    reports.append(report)
                except Exception as e:
                    self._logger.warning(f"Failed to parse position report: {e}")
                    continue

        except Exception as e:
            self._logger.error(f"Failed to generate position status reports: {e}")

        return reports

    async def _register_all_currencies(self):
        """Dynamically register currencies from Rakuten Wallet."""
        from nautilus_trader.model.currencies import Currency
        try:
            from nautilus_trader.model.enums import CurrencyType
        except ImportError:
            CurrencyType = None

        try:
            symbols_json = await self._rest_client.get_symbols_py()
            data = json.loads(symbols_json)
            if isinstance(data, dict):
                symbols = data.get("data", [])
            elif isinstance(data, list):
                symbols = data
            else:
                return

            codes = set()
            for s in symbols:
                symbol_name = s.get("symbolName", "")
                if "_" in str(symbol_name):
                    parts = symbol_name.split("_")
                    codes.add(parts[0].upper())
                    if len(parts) > 1:
                        codes.add(parts[1].upper())
                elif "/" in str(symbol_name):
                    parts = symbol_name.split("/")
                    codes.add(parts[0].upper())
                    if len(parts) > 1:
                        codes.add(parts[1].upper())
                else:
                    codes.add(symbol_name.upper())
            codes.add("JPY")

            from nautilus_trader.model import currencies as model_currencies
            added_count = 0

            for code in codes:
                if hasattr(self._instrument_provider, "currency"):
                    if self._instrument_provider.currency(code):
                        continue

                if getattr(model_currencies, code, None):
                    continue

                try:
                    ctype = CurrencyType.CRYPTO
                    if code in ("JPY", "USD", "EUR"):
                        ctype = CurrencyType.FIAT

                    currency = Currency(code, 8, 0, code, ctype)

                    if hasattr(self._instrument_provider, "add_currency"):
                        self._instrument_provider.add_currency(currency)
                        added_count += 1
                except Exception as e:
                    self.log.warning(f"Could not add currency {code}: {e}")

            if added_count > 0:
                self.log.info(f"Dynamically registered {added_count} currencies from Rakuten Wallet")

        except Exception as e:
            self.log.error(f"Failed to register currencies: {e}")
