# Hashline Performance Benchmarks

Measured on Apple Silicon (M-series). Rust binary: `hashline v0.1.3` (release build). Bun: `v1.3.7`.

All in-process numbers use 50 iterations with 3 warmup rounds. CLI wall-clock numbers use 20 runs averaged. Hash parity is verified byte-for-byte against `Bun.hash.xxHash32`.

---

## Hash Parity

Rust (`xxhash_rust::xxh32`, seed=0, mod 256) and Bun (`Bun.hash.xxHash32`, seed=0, mod 256) produce **identical output** for all tested inputs. Verified across 10 hand-crafted vectors (see `tests/integration.rs::hash_compat_bun_vectors`) and a full 10-line parity file:

| Line content | Rust | Bun |
|---|---|---|
| `let var_0 = compute_something(0, "arg");` | `23` | `23` |
| `let var_42 = compute_something(294, "arg");` | `3a` | `3a` |
| `let var_999 = compute_something(6993, "arg");` | `78` | `78` |
| `function hello() {` | `42` | `42` |
| `  return "world";` | `5e` | `5e` |
| `}` | `18` | `18` |
| _(empty)_ | `05` | `05` |
| `const x = 1 + 2;` | `2b` | `2b` |
| `// comment` | `48` | `48` |
| `let x: Vec<String> = vec![];` | `39` | `39` |

---

## In-Process Throughput (library API)

### `format_hashlines` (read + hash + format entire file)

| Lines | Rust (µs) | Rust (lines/sec) | Rust (MB/s) | Bun (µs) | Bun (lines/sec) | Bun (MB/s) | Speedup |
|------:|----------:|-----------------:|------------:|---------:|----------------:|-----------:|--------:|
| 100 | 37 | 2,718,000 | 124 | 55 | 1,820,000 | 83 | **1.5×** |
| 1,000 | 403 | 2,479,000 | 118 | 370 | 2,703,000 | 128 | 0.9× |
| 10,000 | 3,251 | 3,076,000 | 152 | 3,922 | 2,550,000 | 126 | **1.2×** |

> Rust and Bun are within 1–1.5× of each other in sustained throughput once JIT is warm. The real difference is startup cost (see below).

### `compute_line_hash` (per line, no I/O)

| Lines | Rust (µs total) | Rust (ns/line) | Bun (µs total) | Bun (ns/line) | Speedup |
|------:|----------------:|---------------:|---------------:|--------------:|--------:|
| 100 | 19 | 191 | 29 | 291 | **1.5×** |
| 1,000 | 193 | 193 | 272 | 272 | **1.4×** |
| 10,000 | 1,982 | 198 | 2,709 | 271 | **1.4×** |

### `apply_hashline_edits` (validate + splice, in-process)

| File size | Edits | Rust (µs) | Notes |
|----------:|------:|----------:|-------|
| 100 lines | 1 | 39 | |
| 100 lines | 5 | 47 | |
| 100 lines | 20 | 77 | |
| 1,000 lines | 1 | 337 | |
| 1,000 lines | 5 | 343 | |
| 1,000 lines | 20 | 376 | |
| 10,000 lines | 1 | 2,915 | |
| 10,000 lines | 5 | 2,920 | |
| 10,000 lines | 20 | 2,954 | |

Edit overhead is small — going from 1 to 20 edits on a 10k-line file adds only ~40 µs. The dominant cost is reading and re-serializing the file content, not the edit logic itself.

### Batched edits on a 1,000-line file (in-process)

| Edits batched | Total (ms) | Per edit (µs) |
|--------------:|-----------:|--------------:|
| 1 | 0.33 | 331 |
| 10 | 0.35 | 35 |
| 50 | 0.43 | 8.7 |
| 100 | 0.53 | 5.3 |

**Batching pays off dramatically.** 100 edits in one call costs 0.53 ms total — 63× cheaper per edit than 100 individual calls.

---

## CLI Wall-Clock (process invocation overhead)

Each invocation starts a fresh process, reads the file from disk, and writes output.

| Operation | File size | Hashline CLI | Bun script | Speedup |
|-----------|----------:|-------------:|-----------:|--------:|
| `read` | 100 lines | **2 ms** | 18 ms | **9×** |
| `read` | 1,000 lines | **3 ms** | 19 ms | **6×** |
| `read` | 10,000 lines | **7 ms** | 22 ms | **3×** |
| `apply` (1 edit) | 1,000 lines | **4 ms** | 17 ms | **4×** |

Bun noop startup alone costs ~10 ms per invocation. Hashline's Rust binary starts in ~1–2 ms.

### 100 sequential `apply` calls (worst case: no batching)

Each call is a separate process invocation: read file → validate anchor → apply edit → write file.

| Tool | 100 calls total | Per call |
|------|----------------:|---------:|
| `hashline apply` | **403 ms** | **4 ms** |
| `bun apply_script` | 1,715 ms | 17 ms |
| **Speedup** | **4.3×** | **4.3×** |

And compared to in-process batched (the ideal case):

| Approach | 100 edits total | Per edit |
|----------|----------------:|---------:|
| `hashline apply` (100 CLI calls, unbatched) | 403 ms | 4 ms |
| `hashline apply` (1 CLI call, 100 edits batched) | ~3 ms | 0.03 ms |
| `bun apply_script` (100 CLI calls) | 1,715 ms | 17 ms |

**Recommendation:** Always batch edits to the same file in a single `hashline apply` call. The atomicity guarantee covers all edits in the batch — if any anchor fails, none are applied.

---

## Summary

| Metric | Value |
|--------|-------|
| Hash parity with Bun/TS | ✅ Exact match |
| Rust startup overhead | ~1–2 ms |
| Bun startup overhead | ~10 ms |
| `hashline read` (1k lines) | 3 ms wall-clock |
| `hashline apply` (1 edit, 1k lines) | 4 ms wall-clock |
| In-process: 100 edits batched (1k lines) | 0.53 ms |
| CLI speedup vs Bun (read) | 6–9× |
| CLI speedup vs Bun (apply) | 4× |
