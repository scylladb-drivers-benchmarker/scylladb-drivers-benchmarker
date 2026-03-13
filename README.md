# SDB — ScyllaDB Drivers Benchmarker

SDB is a tool for benchmarking, comparing, and visualizing the performance of different implementations.

## Quick Start

SDB has three main functions:

1. **`run`**: Execute a benchmark run and save the results.
2. **`plot`**: Visualize and compare results from previous runs.
3. **`database`**: Access database - print its content or remove it.

### Example usage

From inside the `tests/cpp_vs_rust_test/cpp` and `tests/cpp_vs_rust_test/rust` directories execute:

```sh
cargo run -- -d ../test.db run regex -b ../config.yml
```

Then, later to graph the results execute (from `tests/cpp_vs_rust_test`):

```sh
cargo run -- -d test.db plot regex -b config.yml \
  --series regex-cpp@cpp/ \
  --series regex-rust@rust/ \
  series
```

Finally it is possible to print data to stdout:

```sh
cargo run -- -d test.db database print
```

possibly applying filters:

```sh
cargo run -- -d test.db database print --benchmark-point 400000:6400000
```

Or remove it:

```sh
cargo run -- -d test.db database drop --benchmark-point=400000:6400000
```

## Logging

By default the application is in logging `info` mode, but this can be changed by setting the `RUST_LOG` environment variable.

SDB uses standard logging modes: `off` to remove any logging, more expressive `debug` and even more verbose `trace`.

### Logging example

```sh
RUST_LOG=off cargo run -- -d ../test.db run regex -b ../config.yml
```

## Definitions

1. benchmark — a platform for evaluating implementations, which consists of:
   - benchmarking points — the sizes for each subsequent measuring run.
   - timeout — optional, the upper limit of how long a process being measured can execute.
2. backend — an implementation being compared. It defines how to invoke it, including:
   - build command — executed once before benchmarking
   - run command — the command being measured. It should take the size of the run as the first and only argument.
3. measurement method — the wrapper method, which invokes the backend. The backend run command with all of it's arguments will be passed as separate arguments to the measuring command, without quoting.
4. command — a program name with specified arguments.

## Supported workflows

### Measurement methods

In general any command printing a single numbers can be passed as a measuring method and plotted on a graph. Both `stdout` and `stderr` are collected together, so the resulting number can be in any of them. This is done to support the unusual behavior of the `time` command.
There are two special measuring methods that get treated differently:

- `time` - measures the elapsed real time
- `perf` - captures events given by `perf-stat` (architecture dependant)
- `flame-graph` - captures and stack-folds the data to be plotted as a flame graph

### Plotting options

Different data requires different plot types to be visualized correctly. Currently supported plot types:

- `series`
  - used for single value outputs (from custom or `time` measuring)
  - generates a linear or logarithmic graph
  - currently supported formats: `png`, `svg` (default)
- `perf-stat`
  - used for the results of a `perf` measurement method
  - generates multiple graphs, one for each requested event
  - currently supported formats: `png`, `svg` (default)
- `flamegraph`
  - used for the results of a `flame-graph` measurement method
  - generates a single file containing all the generated graphs; if specified it is also possible to generate singular graphs
  - currently supported formats: `html` (default)

## Configuration files

The benchmark and backend must be specified in the configuration files, in the YAML format. Each file can store a list of configurations.

### Example benchmark configuration file

```YAML
benchmarks:
  - name: regex
    starting-step: 10
    no-steps: 4
    step-progress: 10
    progress-type: multiplicative
  - name: dictionary
    starting-step: 1000000
    no-steps: 6
    step-progress: 1000000
    progress-type: additive
```

### Example backend configuration file

```YAML
backends:
  - name: regex-cpp
    benchmark-name: regex
    build-command: make regex
    run-command: ./regex
  - name: dictionary-cpp
    benchmark-name: dictionary
    build-command: make dictionary
    run-command: ./dictionary
```

Additionally one may specify the aliasing config to ease the usage of the application, by naming the paths to the global resources used by the benchmarker. Each of them can of course be overwritten by passing a matching command line argument.

The path to the aliasing config should be available in an environment variable named `SDB_CONFIG`. This makes it possible to set the configuration once, permanently.

### Example of a full aliasing config

```YAML
db-path: /home/abc/sdb/db
repo-path:
  rust: /home/abc/scylla-db-rust
  nodejs: /home/abc/nodejs-rs-driver
flame-path: /home/abc/FlameGraph/
store-dir: /home/abc/store/
benchmark-config: /home/abc/bench.yml
```

## In-depth CLI

The benchmarker accepts following options:

- `-d`, `--db-path` — the path to the database location, default is `benchmarker.db` located in home directory.

Subcommand should be provided after database:

### Run subcommand

Executes, measures, and stores to the database the results of the measurements. It should be invoked from the inside of the repository holding the application being measured. After `run`, a benchmark named should be passed.

- `-b`, `--benchmark-config-path` — the path to the configuration file of the benchmark
- `-B`, `--backend-config-path` — the path to the configuration file of the backend
- `-M`, `--benchmark-mode` — `used_cached`(default, uses data from database) or `force-rerun`(overrides database data).
- The subcommand used profile should be passed next. We support the following:
  - time (default) — measures lapsed real (wall clock) time used by the process, in seconds.
  - perf-stat — gathers the performance counter statistics.
  - command - Custom measuring command. Should output exactly one number on either `stdout` or `stderr`.
  - flame-graph — Captures the data given by `perf record`, processes it and collapses the stack. It can be customized using the following options:
    - `-r`, `--flame-repo` — the path to the flame-graph repository of Brendan Gregg.
    - `-f`, `--frequency` — the frequency at which it the run command be profiled.
    - `-s`, `--store-dir` — the directory in which to store the folded results.

### Plot subcommand

Visualizes and compares the results of previous `run`s, reading them from the database.
After `plot` a benchmark name must be passed.

- `-b`, `--benchmark-setup` — The path to the benchmark configuration file, or a comma-separated list of benchmark points (e.g. `1000,2000,4000`). If omitted, the benchmark is looked up by name in the path given by `benchmark-config` in the aliasing config.
- `-o`, `--output` — The path where the plot should be saved. The file extension **implies** the output format (see [Plotting options](#plotting-options)). Defaults to `out.svg` for series/perf-stat and `out.html` for flame-graph.
- `--series <BACKEND@REPO[:REF[=ALIAS]]>` — Adds one data series to the plot. Repeat the flag to overlay multiple series on the same chart.
  - `BACKEND` — the backend name as stored in the database (e.g. `regex-cpp`).
  - `REPO` — a filesystem path to a git repository, or a short alias defined in the aliasing config under `repo-path`.
  - `REF` *(optional)* — a git ref (commit hash, branch, or tag) to use. Defaults to `HEAD`.
  - `ALIAS` *(optional)* — the label shown on the plot legend. Defaults to `REF`.

  **Examples:**

  ```sh
  # Two backends, both at HEAD of their respective repositories:
  --series regex-cpp@./cpp --series regex-rust@./rust

  # A specific commit, with a human-readable alias on the plot:
  --series regex-cpp@./cpp:d7c7310fb60fd659bfa2ff3ff131973a822a91a4=v2.1.0

  # The same backend at two different commits (aliased for clarity):
  --series regex-cpp@./cpp:HEAD=main --series regex-cpp@./cpp:v1.0.0

  # Using a repo-path alias from the aliasing config:
  --series regex-cpp@cpp-driver:HEAD
  ```

- Plot type (subcommand) and its flags must follow all other options:
  - `series` — line chart for single-value outputs (`time` or custom command).
    - `-m`, `--measurement-method` — measurement method: `time` (default), `perf`, `flame-graph`, or a custom command.
    - `-v`, `--visualization-kind` — `linear` (default) or `log` scale.
  - `perf-stat` — one chart per requested `perf` event.
    - `-e`, `--events <event1,event2,...>` — comma-separated list of `perf` event names (e.g. `task-clock,page-faults`). At least one is required. Event names are platform-dependent.
  - `flamegraph` — HTML file containing embedded flame graphs.
    - `-a`, `--artifacts-dir` — directory where individual `.svg` flame graphs are saved. Omit to skip saving them.
    - `-f`, `--flame-repo` — path to the [FlameGraph](https://github.com/brendangregg/FlameGraph) repository. Required unless set in the aliasing config (`flame-path`).

**Full example — comparing three drivers at a specific commit:**

```sh
sdb -d results.db plot regex \
  -b benchmark/runner-config/config.yml \
  --series regex-cpp@benchmark/runner-config/cpp-driver:d7c7310=v2.1 \
  --series regex-java@benchmark/runner-config/java-driver:d7c7310=v2.1 \
  --series regex-rust@benchmark/runner-config/rust-driver:HEAD \
  series
```

### Database subcommands

Facilitates direct access to the underlying database. This was especially useful in debugging or generally developing this application

There are two possible subcommand for `database`:

- `drop` — erases the selected data from the database.
- `print` — outputs the selected data from the database.

The data can be filtered using the following options:

- `--commit-hash` — accepts list of accepted commit hashes divided by `:`, empty (or lack of argument) means it is not restricted.
- `--benchmark-name` — in same format restricts benchmark names.
- `--measurement-method` — in same format restricts measurement methods.
- `--benchmark-point` — in same format restricts benchmark points. Provided elements must be non negative integers.

## Authors

- Paweł Mieszkowski
- Paweł Zalewski
- Krzysztof Hałubek
