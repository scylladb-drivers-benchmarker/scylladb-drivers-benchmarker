# SDB — ScyllaDB Drivers Benchmarker

SDB is a tool for benchmarking, comparing, and visualizing the performance of ScyllaDB/Cassandra
drivers. It works in tandem with the
[scylladb-drivers-benchmarks](https://github.com/scylladb/scylladb-drivers-benchmarks) repository,
which holds the workload implementations and configuration (see its README for the repository
layout and the contracts between the two).

SDB is invocable from any working directory — all paths (the driver repository, the benchmarks
repository, the results database) are passed explicitly.

## Quick Start

SDB has three main functions:

1. **`run`**: Execute benchmarks for one driver and save the results.
2. **`plot`**: Visualize and compare results from previous runs.
3. **`database`**: Access the results database — print its content or remove entries.

### Example usage

```sh
# Benchmark a local driver checkout (its repo must contain benchmark-config.yml):
sdb -d results.db run \
    --driver-path ~/scylla-rust-driver \
    --benchmarks-path ~/scylladb-drivers-benchmarks \
    --scenario insert \
    time

# Benchmark a published driver version:
sdb -d results.db run \
    --driver published:nodejs:cassandra-driver@4.8.0 \
    --benchmarks-path ~/scylladb-drivers-benchmarks \
    time

# Compare two branches of the same driver on a plot:
sdb -d results.db plot insert \
    -b ~/scylladb-drivers-benchmarks/scenarios/config.yml \
    --series rust-driver@~/scylla-rust-driver:main=baseline \
    --series rust-driver@~/scylla-rust-driver:my-feature=candidate \
    series -q throughput

# Inspect or clean the database:
sdb -d results.db database print --driver-name rust-driver
sdb -d results.db database drop --benchmark-point=400000:6400000
```

## How a run works

For every `run` invocation SDB:

1. Resolves the **driver under test**: from `--driver-path <repo>` (reading the repo's
   `benchmark-config.yml`: driver name, api, package, package-path) or from
   `--driver published:<api>:<package>@<version>`. The stored version identity is the repo's
   `HEAD` commit hash (suffixed `-dirty` when the working tree has uncommitted changes) or
   `v<version>`.
2. Loads `apis/<api>/api-config.yml` from the benchmarks repository. All commands below run with
   that api directory as their working directory.
3. Runs the api's **env-prepare** script (links the driver into the benchmark project) and the
   **build command** — once, unmeasured.
4. For every selected scenario, for every benchmark point, for each of `num-runs` repetitions,
   invokes the api's run command three times (parameters passed via env vars —
   `PHASE`, `BENCHMARK`, `STEP`, `PARAM_MODE`, `DRIVER_PACKAGE`):
   - `PHASE=prepare` — unmeasured (schema creation, data seeding); a failure aborts the point;
   - `PHASE=run` — **measured** (wrapped in `time`, `perf stat`, `perf record`, or a custom command);
   - `PHASE=teardown` — unmeasured, best-effort cleanup.
5. Stores the result (for `time`: mean and stddev over `num-runs`) in the SQLite database.
6. Runs the api's **env-cleanup** script, also when benchmarking fails.

A failing point aborts the run with a non-zero exit status unless `--keep-going` is given
(then it is logged and simply absent from the database).

## Logging

By default the application logs at `info`; set `RUST_LOG` (`off`, `debug`, `trace`) to change it.

## In-depth CLI

Global option:

- `-d`, `--db-path` — the path to the results database, default `~/SDB_benchmarker.db`.

### Run subcommand

- `--driver-path <path>` — local driver repository (must contain `benchmark-config.yml`), **or**
- `--driver published:<api>:<package>@<version>` — published driver (exactly one of the two is required).
- `--driver-name <name>` — override for the recorded driver name (default: `driver-name` from
  `benchmark-config.yml`, or the package name for published drivers). Handy for recording a
  published version under the same series name as the repository, e.g.
  `--driver published:rust-v1:scylla@1.7.0 --driver-name rust-driver`.
- `-p`, `--benchmarks-path <path>` — the benchmarks repository (required).
- `-s`, `--scenario <name>` — scenario to run; repeatable. Default: all scenarios.
- `-b`, `--scenarios-config <path>` — override for `<benchmarks-path>/scenarios/config.yml`.
- `-M`, `--benchmark-mode` — `use-cached` (default; skips points already in the database) or
  `force-rerun` (deletes and re-measures them).
- `--keep-going` — continue past failing points instead of aborting.
- The measurement method is the trailing subcommand:
  - `time` (default) — elapsed real (wall clock) time of the run phase, in seconds.
  - `perf-stat` — performance counter statistics.
  - `command <cmd>` — custom measuring command wrapping the run command; must output exactly one
    number on `stdout`/`stderr`.
  - `flame-graph` — captures `perf record` data of the run phase and collapses the stack:
    - `-r`, `--flame-repo` — path to Brendan Gregg's [FlameGraph](https://github.com/brendangregg/FlameGraph) repository (required).
    - `-f`, `--frequency` — profiling frequency.
    - `-s`, `--store-dir` — directory in which to store the folded results (required).

### Plot subcommand

Visualizes and compares the results of previous `run`s, reading them from the database.
After `plot` a benchmark name may be passed; if omitted, all benchmarks from the config are
rendered into a grid.

- `-b`, `--benchmark-setup` — the path to the scenarios configuration file, or a comma-separated
  list of benchmark points (e.g. `1000,2000,4000`). Required.
- `-o`, `--output` — where to save the plot; the file extension implies the format
  (`png`/`svg` for series and perf-stat, `html` for flame graphs). Defaults to `out.svg` / `out.html`.
- `--series <DRIVER@REPO[:REF][=ALIAS]>` — adds one data series; repeat to overlay several.
  - `DRIVER` — the driver name as stored in the database (from `benchmark-config.yml`, or the
    package name for published drivers).
  - `REPO` — a filesystem path to a git repository — `REF` (branch, tag, or hash; default `HEAD`)
    is then resolved with git inside it. If `REPO` is **not** an existing directory it is taken
    literally as the stored version identity (a full commit hash, `<hash>-dirty`, or `v4.8.0`).
  - `ALIAS` — the label shown in the plot legend (defaults to `REF`, or a shortened literal id).

  ```sh
  # Cross-branch: same driver, two refs of its repository:
  --series rust-driver@~/scylla-rust-driver:main=baseline \
  --series rust-driver@~/scylla-rust-driver:my-feature=candidate

  # Cross-driver, one of them published (literal identities):
  --series rust-driver@1a2b3c...=rust --series cassandra-driver@v4.8.0=dsx
  ```

- The plot type (subcommand) follows all other options:
  - `series` — chart of single-value outputs.
    - `-m`, `--measurement-method` — `time` (default), `perf`, or a custom command.
    - `-q`, `--quantity` — what goes on the y axis:
      - `throughput` — benchmark point per second, derived from the measured duration, drawn as
        **grouped columns**: one group per benchmark point, one column per series, with the legend
        in a strip beside the chart. **This is the recommended view for comparing drivers**: the
        columns of one group are directly comparable, and throughput is roughly flat in the input
        size, so a linear axis stays readable even across an order of magnitude between drivers.
        The unit is points per second — for `ser`/`deser` a point is not a query count, hence the
        generic name. Error bars are not drawn yet.
      - `time` (default) — the measured value itself, drawn as one **line** per series against the
        input size. Use it to see how a driver scales, or when the measured value is not a
        duration that throughput could be derived from.
    - `-v`, `--visualization-kind` — scaling of the y axis: `linear` (default) or `log`. Combines
      freely with `-q`; `-q throughput -v log` is a column chart on a logarithmic axis, whose
      columns then stand on the bottom of the axis rather than on zero.
  - `perf-stat` — one chart per requested `perf` event.
    - `-e`, `--events <e1,e2,...>` — required; event names are platform-dependent.
  - `flame-graph` — HTML file containing embedded flame graphs.
    - `-a`, `--artifacts-dir` — directory where individual `.svg` flame graphs are saved (optional).
    - `-f`, `--flame-repo` — path to the FlameGraph repository (required).

### Database subcommands

- `print` — outputs the selected data.
- `drop` — erases the selected data.

Filters (each accepts a `:`-separated list; absent means unrestricted): `--commit-hash`,
`--benchmark-name`, `--driver-name`, `--benchmark-point`, `--measurement-method`.

### Results database

One SQLite table keyed by
`(commit_hash, benchmark_name, driver_name, benchmark_point, measurement_method)`, with
provenance columns (`api`, `benchmarks_commit`, `timestamp`) recorded for traceability.
Use the same database file for every run you want to compare on one plot.

## Authors

- Paweł Mieszkowski
- Paweł Zalewski
- Krzysztof Hałubek
