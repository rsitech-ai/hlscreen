# Third-Party Notices

The Rust dependency license texts for the release dependency graph are in
[`THIRD_PARTY_LICENSES.txt`](THIRD_PARTY_LICENSES.txt). That file is generated
deterministically from `Cargo.lock` with cargo-about 0.9.1.

## cfg_aliases 0.1.1 and 0.2.1

Both locked versions package the same `NOTICES.md` for code derived from
`tectonic_cfg_support`. The exact shared notice is preserved at
[`third_party/notices/cfg_aliases-0.1.1-and-0.2.1-NOTICES.md`](third_party/notices/cfg_aliases-0.1.1-and-0.2.1-NOTICES.md)
and verbatim below:

```text
# 3rd Party Notices

The `cfg_aliases!` macro uses a lot of the code from [`tectonic_cfg_support::target_cfg!`] macro which is under the following license:

[`tectonic_cfg_support::target_cfg!`]: https://github.com/tectonic-typesetting/tectonic/blob/f2439b936470ad27bdf92882064bc4702ee01899/cfg_support/src/lib.rs#L166

    tectonic_cfg_support is licensed under the MIT License.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the “Software”), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
---
```

## Apache Parquet 59.1.0

The release graph includes Apache Parquet 59.1.0. Its packaged `NOTICE.txt` is
preserved at
[`third_party/notices/parquet-59.1.0-NOTICE.txt`](third_party/notices/parquet-59.1.0-NOTICE.txt)
and verbatim below as required by the Apache License 2.0. The locked-graph
checker rejects any new or changed packaged `NOTICE` file until this document
and `third_party/notices/manifest.json` are reviewed and updated.

```text
Apache Arrow
Copyright 2016-2026 The Apache Software Foundation

This product includes software developed at
The Apache Software Foundation (http://www.apache.org/).

This product includes software from the chronoutil crate (MIT)
 * Copyright (c) 2020-2022 Oliver Margetts
 * https://github.com/olliemath/chronoutil

This product includes software from the compact-thrift project (Apache 2.0)
 * Copyright Jörn Horstmann
 * https://github.com/jhorstmann/compact-thrift
```
