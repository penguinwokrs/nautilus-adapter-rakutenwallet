from nautilus_rakutenwallet.constants import (
    RAKUTENW_VENUE,
    ORDER_STATUS_MAP,
    ORDER_SIDE_MAP,
    ORDER_TYPE_MAP,
    NAUTILUS_TO_RW_ORDER_TYPE,
    BAR_SPEC_TO_RW_INTERVAL,
    BAR_POLL_INTERVALS,
    ERROR_CODES,
)


class TestVenue:
    def test_venue_value(self):
        assert str(RAKUTENW_VENUE) == "RAKUTENW"


class TestOrderStatusMap:
    def test_working_order(self):
        assert ORDER_STATUS_MAP["WORKING_ORDER"] == "ACCEPTED"

    def test_partial_fill(self):
        assert ORDER_STATUS_MAP["PARTIAL_FILL"] == "PARTIALLY_FILLED"


class TestOrderSideMap:
    def test_buy(self):
        assert ORDER_SIDE_MAP["BUY"] == "BUY"

    def test_sell(self):
        assert ORDER_SIDE_MAP["SELL"] == "SELL"


class TestOrderTypeMap:
    def test_market(self):
        assert ORDER_TYPE_MAP["MARKET"] == "MARKET"

    def test_limit(self):
        assert ORDER_TYPE_MAP["LIMIT"] == "LIMIT"

    def test_stop(self):
        assert ORDER_TYPE_MAP["STOP"] == "STOP_MARKET"


class TestReverseOrderTypeMap:
    def test_market(self):
        assert NAUTILUS_TO_RW_ORDER_TYPE["MARKET"] == "MARKET"

    def test_limit(self):
        assert NAUTILUS_TO_RW_ORDER_TYPE["LIMIT"] == "LIMIT"

    def test_stop_market(self):
        assert NAUTILUS_TO_RW_ORDER_TYPE["STOP_MARKET"] == "STOP"


class TestBarSpecMapping:
    def test_1min(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(1, "MINUTE")] == "PT1M"

    def test_5min(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(5, "MINUTE")] == "PT5M"

    def test_1hour(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(1, "HOUR")] == "PT1H"

    def test_1day(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(1, "DAY")] == "P1D"

    def test_1week(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(1, "WEEK")] == "P1W"

    def test_1month(self):
        assert BAR_SPEC_TO_RW_INTERVAL[(1, "MONTH")] == "P1M"


class TestBarPollIntervals:
    def test_all_intervals_have_poll_time(self):
        for interval in BAR_SPEC_TO_RW_INTERVAL.values():
            assert interval in BAR_POLL_INTERVALS


class TestErrorCodes:
    def test_system_error(self):
        assert ERROR_CODES[10001] == "System error"

    def test_maintenance(self):
        assert ERROR_CODES[10005] == "In maintenance"

    def test_too_many_requests(self):
        assert ERROR_CODES[20010] == "Too many requests"

    def test_insufficient_margin(self):
        assert ERROR_CODES[50048] == "Insufficient usable margin"
