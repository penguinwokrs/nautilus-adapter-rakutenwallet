from decimal import Decimal
from nautilus_rakutenwallet.types import (
    RakutenwOrderStatus,
    RakutenwOrderSide,
    RakutenwOrderType,
    RakutenwOrderBehavior,
    RakutenwOrderPattern,
    RakutenwCloseBehavior,
    RakutenwOrderInfo,
    RakutenwTradeRecord,
    RakutenwAsset,
    RakutenwEquityData,
)


class TestRakutenwOrderStatus:
    def test_from_str_working_order(self):
        assert RakutenwOrderStatus.from_str("WORKING_ORDER") == RakutenwOrderStatus.WORKING_ORDER

    def test_from_str_partial_fill(self):
        assert RakutenwOrderStatus.from_str("PARTIAL_FILL") == RakutenwOrderStatus.PARTIAL_FILL

    def test_from_str_unknown_defaults_working(self):
        assert RakutenwOrderStatus.from_str("UNKNOWN") == RakutenwOrderStatus.WORKING_ORDER


class TestRakutenwOrderSide:
    def test_from_str_buy(self):
        assert RakutenwOrderSide.from_str("BUY") == RakutenwOrderSide.BUY

    def test_from_str_sell(self):
        assert RakutenwOrderSide.from_str("SELL") == RakutenwOrderSide.SELL

    def test_from_str_case_insensitive(self):
        assert RakutenwOrderSide.from_str("buy") == RakutenwOrderSide.BUY


class TestRakutenwOrderType:
    def test_from_str_market(self):
        assert RakutenwOrderType.from_str("MARKET") == RakutenwOrderType.MARKET

    def test_from_str_limit(self):
        assert RakutenwOrderType.from_str("LIMIT") == RakutenwOrderType.LIMIT

    def test_from_str_stop(self):
        assert RakutenwOrderType.from_str("STOP") == RakutenwOrderType.STOP

    def test_from_str_unknown_defaults_market(self):
        assert RakutenwOrderType.from_str("UNKNOWN") == RakutenwOrderType.MARKET


class TestRakutenwOrderBehavior:
    def test_open(self):
        assert RakutenwOrderBehavior.OPEN.value == "OPEN"

    def test_close(self):
        assert RakutenwOrderBehavior.CLOSE.value == "CLOSE"


class TestRakutenwOrderPattern:
    def test_normal(self):
        assert RakutenwOrderPattern.NORMAL.value == "NORMAL"

    def test_oco(self):
        assert RakutenwOrderPattern.OCO.value == "OCO"

    def test_ifd(self):
        assert RakutenwOrderPattern.IFD.value == "IFD"

    def test_ifd_oco(self):
        assert RakutenwOrderPattern.IFD_OCO.value == "IFD_OCO"


class TestRakutenwCloseBehavior:
    def test_cross(self):
        assert RakutenwCloseBehavior.CROSS.value == "CROSS"

    def test_fifo(self):
        assert RakutenwCloseBehavior.FIFO.value == "FIFO"


class TestRakutenwOrderInfo:
    def test_is_open_working_order(self):
        order = RakutenwOrderInfo(
            order_id="123",
            symbol_id="1",
            order_behavior=RakutenwOrderBehavior.OPEN,
            side=RakutenwOrderSide.BUY,
            order_pattern=RakutenwOrderPattern.NORMAL,
            order_type=RakutenwOrderType.LIMIT,
            amount=Decimal("0.01"),
            remaining_amount=Decimal("0.01"),
            executed_amount=Decimal("0"),
            price=Decimal("5000000"),
            leverage=Decimal("2"),
            status=RakutenwOrderStatus.WORKING_ORDER,
            timestamp=None,
        )
        assert order.is_open is True

    def test_is_open_partial_fill(self):
        order = RakutenwOrderInfo(
            order_id="123",
            symbol_id="1",
            order_behavior=RakutenwOrderBehavior.OPEN,
            side=RakutenwOrderSide.BUY,
            order_pattern=RakutenwOrderPattern.NORMAL,
            order_type=RakutenwOrderType.LIMIT,
            amount=Decimal("0.01"),
            remaining_amount=Decimal("0.005"),
            executed_amount=Decimal("0.005"),
            price=Decimal("5000000"),
            leverage=Decimal("2"),
            status=RakutenwOrderStatus.PARTIAL_FILL,
            timestamp=None,
        )
        assert order.is_open is True


class TestRakutenwAsset:
    def test_creation(self):
        asset = RakutenwAsset(
            currency_name="JPY",
            onhand_amount=Decimal("1000000"),
        )
        assert asset.currency_name == "JPY"
        assert asset.onhand_amount == Decimal("1000000")


class TestRakutenwEquityData:
    def test_creation(self):
        equity = RakutenwEquityData(
            floating_profit=Decimal("-500"),
            floating_position_fee=Decimal("-100"),
            remaining_floating_position_fee=Decimal("-50"),
            floating_trade_fee=Decimal("-200"),
            floating_profit_all=Decimal("-850"),
            used_margin=Decimal("100000"),
            necessary_margin=Decimal("100000"),
            balance=Decimal("1000000"),
            equity=Decimal("999150"),
            margin_maintenance_percent=Decimal("999"),
            usable_amount=Decimal("800000"),
            withdrawable_amount=Decimal("700000"),
            withdrawal_amount_reserved=Decimal("0"),
        )
        assert equity.usable_amount == Decimal("800000")
        assert equity.used_margin == Decimal("100000")
        assert equity.balance == Decimal("1000000")
        assert equity.equity == Decimal("999150")
        assert equity.floating_profit_all == Decimal("-850")
