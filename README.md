pathrouter
======

[![Crates.io](https://img.shields.io/crates/v/pathrouter)](https://crates.io/crates/pathrouter)
[![Documentation](https://docs.rs/pathrouter/badge.svg)](https://docs.rs/pathrouter)
[![Crates.io](https://img.shields.io/crates/l/pathrouter)](LICENSE)
![Rust](https://github.com/zzzdong/pathrouter/workflows/Rust/badge.svg)

## Overview

[`pathrouter`] is a simple router.

## Usage

```rust
use pathrouter::{Router, Params};

let mut router = Router::new();

router.add("/posts", "posts");
router.add("/posts/:post_id", "post");

let (endpoint, params) = router.route("/posts/1").unwrap();
```

## License

This project is licensed under the [MIT license](LICENSE).
