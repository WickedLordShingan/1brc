# one_brc

My Rust attempt at the [One Billion Row Challenge](https://github.com/gunnarmorling/1brc): read a ~16 GB file of `station;temperature` rows and print min/mean/max per station.

I did this mostly to learn how to actually optimize something, using `perf` to find out what's slow instead of guessing.

Timings on my ThinkPad E14 (warm cache, 1B rows): about 8.8 s with 413 stations and about 20 s with 10k stations.

## Build

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Release profile in `Cargo.toml`:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
debug = true   # symbols, so perf can show function names
```

## Generating measurements

`create.py` builds `measurements.txt` from the station names in `weather_stations.csv`:

```bash
python create.py 1_000_000_000
```

By default it draws from 10k names. For a 413-station file, change the sampling to `random.sample(weather_station_names, 413)`, seed it with `random.seed(42)`, and return `sorted(set(...))` from the name loader so the file is reproducible. Check on a small file first:

```bash
python create.py 1_000_000
cut -d';' -f1 measurements.txt | sort -u | wc -l   # should print 413
```

The program reads `./create_file/measurements.txt`, so move the file there or edit `file_name` in `main.rs`.

## Profiling

```bash
perf stat -e branches,branch-misses,L1-dcache-load-misses ./target/release/one_brc
perf record -g ./target/release/one_brc
perf report
perf annotate one_brc::insert_reading
```

## Optimizations

1. **mmap the file** instead of reading it, so there are no buffers or copies.
2. **One thread per core**, each on its own chunk. Chunk boundaries are moved forward to the next `\n`. Every thread has its own hash table and they're merged at the end, so there are no locks.
3. **Temperatures as integers.** `12.3` is stored as `123` in an `i16`. Floats only appear when printing.
4. **SWAR search for `;`.** Load 8 bytes into a `u64` and find the delimiter with bit tricks instead of checking byte by byte.
5. **Right-sized hash table.** The first version allocated 2^24 slots per thread (about 640 MB each), and most of the runtime went into filling that memory. The hash is only 16 bits, so almost all of it was unreachable anyway. Shrinking it to fit the data fixed that.
6. **Compare names without touching the file.** Each table entry stores the first 8 bytes of the name and its length. Names of 8 bytes or fewer are matched with integer compares only. Longer names fall back to a full byte compare.

## Things I found along the way

- 413 stations vs 10k stations is the same code but ~2.7x slower, because L1 misses went up ~28x (86M to 2.5B). The small file fits in cache and hides a lot of problems.
- Branch misses are around 1.5 to 2 per row. The sign, the digit loop and the probe loop are the culprits.

## Known issues

- `TABLE_SIZE` must be bigger than the number of distinct stations, or the probe loop never ends. `1 << 11` is fine for 413 stations; use `1 << 15` for 10k.
- The hash is still byte-at-a-time FNV.
- Chunks are equal-sized, so one slow core holds everything up.
- Output is one `name=min/mean/max` line per station, not the official `{...}` format.
- `create.py` uses uniform temperatures in [-99.9, 99.9], so about half the rows are negative. That's more than the official data, so don't compare my times to the leaderboard.

## TODO

- Fixed-offset temperature parsing, branchless sign
- Faster hash built from the word already loaded for the `;` search
- Smaller table entries (32 bytes)
- Smaller chunks handed out dynamically
- Process a few independent rows at once to hide cache latency
