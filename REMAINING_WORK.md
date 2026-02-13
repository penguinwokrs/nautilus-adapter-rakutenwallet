# 楽天ウォレット アダプタ — 残作業・テスト計画

## Context

楽天ウォレット NautilusTrader アダプタの初期実装は完了済み（Rust 6ファイル + Python 8ファイル、51テスト全パス）。
本ドキュメントでは、コード品質・テストカバレッジ・運用安定性の観点から残課題を洗い出し、優先度別に整理する。

---

## 1. バグ・クリティカル修正（優先度: Critical）

### 1-1. `_order_states` メモリリーク
- **ファイル**: `nautilus_rakutenwallet/execution.py`
- **問題**: `_order_states` dict に完了済み注文が蓄積され続ける。長時間運用でメモリ増大。
- **対応**: 注文完了時（filled/canceled）に `_order_states` からエントリを削除する。TTL付きキャッシュまたは最大サイズ制限を設ける。

### 1-2. 注文失敗時に OrderRejected イベントが生成されない
- **ファイル**: `nautilus_rakutenwallet/execution.py` (`_submit_order`, `_cancel_order`, `_modify_order`)
- **問題**: REST API エラー時に `except Exception` でログ出力のみ。NautilusTrader のストラテジ側にエラーが通知されない。
- **対応**: `self.generate_order_rejected()` / 適切なイベントを生成する。

### 1-3. ExecutionClient の graceful disconnect 未実装
- **ファイル**: `nautilus_rakutenwallet/execution.py` (`_disconnect`)
- **問題**: `_disconnect` がログ出力のみで、Rust 側の polling loop を停止しない。
- **対応**: Rust 側の `shutdown` AtomicBool を true に設定するメソッドを呼び出す（disconnect メソッドの追加）。

### 1-4. 空文字列 API クレデンシャルのフォールバック
- **ファイル**: `nautilus_rakutenwallet/data.py:55-56`, `execution.py:66-67`
- **問題**: `self.config.api_key or ""` で空文字列を Rust 側に渡す。認証エラーが遅延して発覚する。
- **対応**: config の `__post_init__` バリデーションが正しく動作することを確認。Rust 側でも空文字チェックを追加。

---

## 2. 機能改善（優先度: High）

### 2-1. QuoteTick の bid_size / ask_size が常に 0
- **ファイル**: `nautilus_rakutenwallet/data.py:139-140`
- **問題**: `Quantity.from_str("0")` がハードコード。楽天 WS の Ticker に数量情報がないため仕方ないが、OrderBook の best bid/ask 数量を利用可能。
- **対応**: OrderBook データから best bid/ask の数量を取得して設定するか、ドキュメントで制約を明記。

### 2-2. generate_order_status_reports の precision ハードコード
- **ファイル**: `nautilus_rakutenwallet/execution.py:528-529`
- **問題**: `Quantity(amount, precision=8)` と `Price(Decimal(price_str), precision=0)` が固定。
- **対応**: instrument の `size_precision` / `price_precision` を参照する。

### 2-3. Config の msgspec 互換性
- **ファイル**: `nautilus_rakutenwallet/config.py`
- **問題**: `__post_init__` は dataclass 用。NautilusTrader 1.222+ は msgspec を使用しているため、バリデーションが呼ばれない可能性がある。
- **対応**: msgspec の `__post_init__` 互換性を確認し、必要に応じて `model_post_init` に変更。

### 2-4. data.py の fetch_instruments / _load_instruments 重複
- **ファイル**: `nautilus_rakutenwallet/data.py:199-265`, `data.py:426-528`
- **問題**: `fetch_instruments` と `_load_instruments` に重複したシンボル解析ロジック。providers.py にも同様のロジック。
- **対応**: providers.py の `_parse_instrument` に統一し、data.py からは InstrumentProvider 経由で取得。

### 2-5. WS 再接続時のジッター追加
- **ファイル**: `src/client/data_client.rs:299-300`
- **問題**: 指数バックオフにランダムジッターがない。複数インスタンスが同時再接続するリスク。
- **対応**: バックオフにランダムジッター（例: ±25%）を追加。

---

## 3. テスト計画

### 3-1. 現状のテストカバレッジ

| モジュール | テスト数 | カバレッジ |
|-----------|---------|-----------|
| config.py | 7 | ✅ 十分 |
| constants.py | ~20 | ✅ 十分 |
| types.py | ~20 | ✅ 十分 |
| providers.py | 0 | ❌ 未テスト |
| data.py | 0 | ❌ 未テスト |
| execution.py | 0 | ❌ 未テスト |
| factories.py | 0 | ❌ 未テスト |
| Rust models | 0 | ❌ 未テスト |
| Rust REST client | 0 | ❌ 未テスト |
| Rust WS client | 0 | ❌ 未テスト |
| **合計** | **51** | **~30%** |

### 3-2. 追加テスト一覧

#### Phase A: ユニットテスト（Mock ベース、API 不要）

**`tests/test_providers.py`** (新規、~15テスト)
- `_parse_instrument` に各種シンボル形式を渡してパース確認
  - `BTC_JPY`, `ETH/JPY`, 数値 symbolId
  - `enabled=False`, `viewOnly=True`, `closeOnly=True` でフィルタ確認
  - tickSize/tradeUnit から precision 計算
  - maxLeverage → margin_init 計算
  - 必須フィールド欠損時の None 返却
- `load_all_async` のモック（REST client の `get_symbols_py` をモック）
- `load_ids_async` のフィルタリング確認
- `load_async` の単一 instrument ロード

**`tests/test_data_client.py`** (新規、~20テスト)
- `_handle_ticker`: Ticker → QuoteTick 変換の正確性
- `_handle_trade`: TradeEntry → TradeTick 変換、AggressorSide マッピング
- `_handle_orderbook`: Depth → OrderBookDeltas 変換（CLEAR + ADD）
- `_subscribe_quote_ticks` / `_unsubscribe_quote_ticks`: 正しいチャネルの subscribe/unsubscribe
- `_subscribe_bars` / `_unsubscribe_bars`: BAR_SPEC_TO_RW_INTERVAL マッピング、未サポートスペック警告
- `_load_instruments`: シンボル API レスポンスからの instrument 登録
- 接続・切断ライフサイクル（Rust client をモック）

**`tests/test_execution_client.py`** (新規、~25テスト)
- `_submit_order`: 各パラメータのタグ抽出（orderBehavior, orderPattern, leverage 等）
  - NORMAL パターン
  - OCO パターン（oco_order_type_1/2, oco_price_1/2）
  - IFD パターン（ifd_close_limit_price, ifd_close_stop_price）
  - TimeInForce → orderExpire マッピング（GTC, DAY）
- `_cancel_order`: symbol_id 取得、venue_order_id 渡し
- `_modify_order`: 必須フィールド（symbol_id, order_pattern, order_type）の取得
- `_handle_poll_message`: OrderUpdate / OrderCompleted イベント処理
- `_process_order_completed`: 約定判定（trades あり→filled, なし→canceled）
- `_process_order_update_from_data`: 部分約定の delta 計算
- `generate_order_status_reports`: レスポンス → OrderStatusReport 変換
- `generate_fill_reports`: レスポンス → FillReport 変換
- `generate_position_status_reports`: レスポンス → PositionStatusReport 変換
- `generate_account_status_reports`: EquityData → AccountState 変換

**`tests/test_factories.py`** (新規、~5テスト)
- `RakutenwDataClientFactory.create`: config 必須チェック、インスタンス生成
- `RakutenwExecutionClientFactory.create`: CacheWrapper 生成、instrument_provider 引き渡し

#### Phase B: Rust ユニットテスト

**`tests/test_rust_models.py`** (新規、~10テスト)
- Rust モデルの Python バインディング確認
  - `RakutenwRestClient` のインスタンス生成
  - `RakutenwDataClient` のインスタンス生成、subscribe/unsubscribe
  - `RakutenwExecutionClient` のインスタンス生成

**`src/` 内 Rust テスト** (新規)
- `src/rate_limit.rs`: TokenBucket の acquire タイミング確認
- `src/model/orderbook.rs`: apply_snapshot、get_top_n の正確性
- `src/error.rs`: エラーコードの PyErr 変換
- `src/client/rest.rs`: 署名生成（HMAC-SHA256）の正確性テスト

#### Phase C: カセットベース統合テスト（VCR パターン）

**`tests/test_integration_public.py`** (新規、~8テスト)
- `get_symbols_py`: シンボル一覧取得 → JSON パース
- `get_ticker_py`: Ticker 取得 → フィールド確認
- `get_orderbook_py`: OrderBook 取得 → asks/bids 確認
- `get_trades_py`: 約定履歴取得
- `get_candlestick_py`: ローソク足取得 → OHLCV 確認

**`tests/test_integration_private.py`** (新規、~10テスト、API キー必須)
- `get_assets_py`: 資産一覧取得
- `get_equity_data_py`: 証拠金情報取得 → 全13フィールド確認
- `get_orders_py`: 注文一覧取得（各フィルタパラメータ）
- `get_positions_py`: ポジション一覧取得
- `get_trades_private_py`: 約定履歴取得（各フィルタパラメータ）
- 注文ライフサイクル: submit → get_orders → cancel（テストネット必要）

### 3-3. テスト実装方法

```python
# Mock パターン例 (test_execution_client.py)
from unittest.mock import AsyncMock, MagicMock, patch

class TestSubmitOrder:
    @pytest.fixture
    def exec_client(self):
        # RakutenwExecutionClient を部分モックで生成
        with patch('nautilus_rakutenwallet.execution.rakutenw') as mock_rw:
            mock_rust_client = MagicMock()
            mock_rust_client.submit_order = AsyncMock(return_value='{"order_id": "123"}')
            mock_rust_client.connect = AsyncMock()
            mock_rw.RakutenwExecutionClient.return_value = mock_rust_client
            mock_rw.RakutenwRestClient.return_value = MagicMock()
            # ... client 構築
            yield client, mock_rust_client

    async def test_submit_normal_order(self, exec_client):
        client, mock_rust = exec_client
        # ... テスト実装
```

```python
# カセットベース統合テスト例 (test_integration_public.py)
@requires_rust_extension
class TestPublicAPI:
    def test_get_symbols(self, vcr):
        async def live_call():
            client = rakutenw.RakutenwRestClient("", "", 10000, None, None)
            return await client.get_symbols_py()

        result = vcr(lambda: asyncio.run(live_call()))
        data = json.loads(result)
        assert isinstance(data, list)
        assert len(data) > 0
        assert "symbolId" in data[0]
```

### 3-4. テスト実行コマンド

```bash
# 全ユニットテスト
pytest tests/ -v --ignore=tests/test_integration_private.py

# 統合テスト（カセット再生）
pytest tests/test_integration_public.py -v

# 統合テスト（API キー必要、ライブ録画）
RAKUTENW_API_KEY=xxx RAKUTENW_API_SECRET=xxx pytest tests/ -v --record-cassettes

# Rust テスト
cargo test

# カバレッジレポート
pytest tests/ -v --cov=nautilus_rakutenwallet --cov-report=html
```

---

## 4. CI/CD 計画

### 4-1. GitHub Actions ワークフロー

**`.github/workflows/ci.yml`** (新規)
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: actions/setup-python@v5
        with: { python-version: "3.12" }
      - run: pip install maturin nautilus-trader pytest pytest-asyncio pytest-cov
      - run: cargo test
      - run: maturin develop
      - run: pytest tests/ -v --cov=nautilus_rakutenwallet --ignore=tests/test_integration_private.py
```

---

## 5. 実装優先度サマリ

| 優先度 | 項目 | 工数目安 |
|--------|------|---------|
| **P0 Critical** | 1-1 メモリリーク修正 | 小 |
| **P0 Critical** | 1-2 OrderRejected イベント生成 | 小 |
| **P0 Critical** | 1-3 graceful disconnect | 小 |
| **P0 Critical** | 1-4 空クレデンシャルチェック | 小 |
| **P1 High** | 3-2 Phase A ユニットテスト（~65テスト） | 大 |
| **P1 High** | 2-2 precision ハードコード修正 | 小 |
| **P1 High** | 2-3 msgspec 互換性確認 | 中 |
| **P1 High** | 2-4 instrument ロジック統一 | 中 |
| **P2 Medium** | 3-2 Phase B Rust テスト（~10テスト） | 中 |
| **P2 Medium** | 3-2 Phase C カセット統合テスト（~18テスト） | 中 |
| **P2 Medium** | 2-1 bid_size/ask_size 改善 | 小 |
| **P2 Medium** | 2-5 WS バックオフジッター | 小 |
| **P3 Low** | 4-1 CI/CD パイプライン構築 | 中 |

---

## 6. 検証方法

1. **ビルド確認**: `cargo test && maturin develop && pytest tests/ -v`
2. **カバレッジ確認**: `pytest --cov=nautilus_rakutenwallet` で 80% 以上を目標
3. **統合確認**: カセットベーステストで Public API レスポンスの互換性確認
4. **長時間運用確認**: `_order_states` のメモリリーク修正後、注文送信→完了のサイクルでメモリが増加しないことを確認
