# Data Format

Raw records preserve exact public market-data payloads with local receive timestamps, connection identity, sequence number, and channel.

Normalized records are derived from raw records and cover trades, top-of-book quotes, asset contexts, all-market mids, candles, data gaps, and recording runs.

Top-of-book metrics must be labeled as `tob_depth_usd` and `tob_imbalance`. They are not full book depth.
