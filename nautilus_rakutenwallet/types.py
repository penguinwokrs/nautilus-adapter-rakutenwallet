# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2024 Penguinworks. All rights reserved.
#
#  Licensed under the GNU General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/gpl-3.0.en.html
# -------------------------------------------------------------------------------------------------
"""
Rakuten Wallet adapter custom types.
"""

from enum import Enum, auto
from dataclasses import dataclass
from decimal import Decimal
from typing import Optional


class RakutenwOrderStatus(Enum):
    WORKING_ORDER = auto()
    PARTIAL_FILL = auto()

    @classmethod
    def from_str(cls, value: str) -> "RakutenwOrderStatus":
        mapping = {
            "WORKING_ORDER": cls.WORKING_ORDER,
            "PARTIAL_FILL": cls.PARTIAL_FILL,
        }
        return mapping.get(value.upper(), cls.WORKING_ORDER)


class RakutenwOrderSide(Enum):
    BUY = "BUY"
    SELL = "SELL"

    @classmethod
    def from_str(cls, value: str) -> "RakutenwOrderSide":
        return cls.BUY if value.upper() == "BUY" else cls.SELL


class RakutenwOrderType(Enum):
    MARKET = "MARKET"
    LIMIT = "LIMIT"
    STOP = "STOP"

    @classmethod
    def from_str(cls, value: str) -> "RakutenwOrderType":
        mapping = {
            "MARKET": cls.MARKET,
            "LIMIT": cls.LIMIT,
            "STOP": cls.STOP,
        }
        return mapping.get(value.upper(), cls.MARKET)


class RakutenwOrderBehavior(Enum):
    OPEN = "OPEN"
    CLOSE = "CLOSE"


class RakutenwOrderPattern(Enum):
    NORMAL = "NORMAL"
    OCO = "OCO"
    IFD = "IFD"
    IFD_OCO = "IFD_OCO"


class RakutenwCloseBehavior(Enum):
    CROSS = "CROSS"  # Allow multi-position
    FIFO = "FIFO"    # No multi-position


@dataclass
class RakutenwOrderInfo:
    order_id: str
    symbol_id: str
    order_behavior: RakutenwOrderBehavior
    side: RakutenwOrderSide
    order_pattern: RakutenwOrderPattern
    order_type: RakutenwOrderType
    amount: Decimal
    remaining_amount: Optional[Decimal]
    executed_amount: Optional[Decimal]
    price: Optional[Decimal]
    leverage: Optional[Decimal]
    status: RakutenwOrderStatus
    timestamp: Optional[str]

    @property
    def is_open(self) -> bool:
        return self.status in (
            RakutenwOrderStatus.WORKING_ORDER,
            RakutenwOrderStatus.PARTIAL_FILL,
        )


@dataclass
class RakutenwTradeRecord:
    trade_id: str
    order_id: Optional[str]
    symbol_id: str
    side: RakutenwOrderSide
    amount: Decimal
    price: Decimal
    fee: Optional[Decimal]
    pnl: Optional[Decimal]
    timestamp: Optional[str]


@dataclass
class RakutenwAsset:
    currency_name: Optional[str]
    onhand_amount: Decimal


@dataclass
class RakutenwEquityData:
    floating_pnl: Optional[Decimal]
    usable_amount: Optional[Decimal]
    withdrawable_amount: Optional[Decimal]
    required_margin_amount: Optional[Decimal]
    order_margin_amount: Optional[Decimal]
    maintenance_rate: Optional[Decimal]
