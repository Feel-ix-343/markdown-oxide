# Large-file benchmark

## Summary

This PR benchmarks only `FPF-Spec.md`, copied into a temporary **one-file vault**, so the numbers track **per-file parser cost** instead of cross-file indexing noise.

| Commit | Rationale | `FPF-Spec.md` initialize |
| --- | --- | --- |
| `46ec5ca` | Baseline before this PR | `64.542s` |
| `61b341f` | Reuse one `Rope` while parsing fenced and inline code blocks instead of rebuilding it for every backtick match | `11.851s` |
| `6d0e8cf` | Reuse the same file `Rope` across the remaining markdown parsers (links, headings, tags, footnotes, indexed blocks, link-ref defs) | `0.921s` |
| `e86720e` | Add `Initialize timings:` startup logs for future diagnosis; no material performance change expected | `0.850s` |

## Reproduce the benchmark

These commands assume **Bash + Python 3**. The script and downloaded `FPF-Spec.md` are copied into the system temp directory first so they survive `git checkout`.

```bash
cd /path/to/markdown-oxide

SCRIPT_COPY="$(python3 - <<'PY'
from pathlib import Path
from tempfile import gettempdir
print(Path(gettempdir()) / 'benchmark_large_file.py')
PY
)"

BENCH_DIR="$(python3 - <<'PY'
from pathlib import Path
from tempfile import gettempdir
print(Path(gettempdir()) / 'markdown-oxide-large-file')
PY
)"

cp docs-dev/benchmark_large_file.py "$SCRIPT_COPY"
python3 "$SCRIPT_COPY" fetch --output-dir "$BENCH_DIR"
FPF_SPEC="$BENCH_DIR/FPF-Spec.md"
```

Benchmark the baseline and each PR commit:

```bash
git checkout 46ec5ca
cargo build --release
python3 "$SCRIPT_COPY" run --binary target/release/markdown-oxide --file "$FPF_SPEC"

git checkout 61b341f
cargo build --release
python3 "$SCRIPT_COPY" run --binary target/release/markdown-oxide --file "$FPF_SPEC"

git checkout 6d0e8cf
cargo build --release
python3 "$SCRIPT_COPY" run --binary target/release/markdown-oxide --file "$FPF_SPEC"

git checkout e86720e
cargo build --release
python3 "$SCRIPT_COPY" run --binary target/release/markdown-oxide --file "$FPF_SPEC"
```

Return to the PR branch when done:

```bash
git switch perf/startup-vault-parsing
```

If your platform builds `markdown-oxide.exe`, pass that path to `--binary` instead.

## Run the built-in cargo perf tests

These perf regression tests are part of the normal test suite now. Running them in `--release`
keeps the timing signal less noisy when you want to inspect them directly.

`46ec5ca` does not contain them yet.

Available from `61b341f` and later:

```bash
cargo test --release test_inline_code_block_perf_regression
```

Available from `6d0e8cf` and later:

```bash
cargo test --release test_md_file_perf_regression
```

On `e86720e` and later, the external benchmark also prints the `Initialize timings:` log line, which is the fastest way to confirm where startup time is spent.
