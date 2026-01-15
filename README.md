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
cargo run -- -d test.db plot regex -b config.yml --from=cpp/:HEAD --from=rust/:HEAD series
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

## Definitions

1. benchmark — a platform for evaluating implementations, which consists of:
    * benchmarking points — the sizes for each subsequent measuring run.
    * timeout — optional, the upper limit of how long a process being measured can execute.
2. backend — an implementation being compared. It defines how to invoke it, including:
    * build command — executed once before benchmarking  
    * run command   — the command being measured. It should take the size of the run as the first and only argument.
3. measurement method — the wrapper method, which invokes the backend. The backend run command with all of it's arguments will be passed as separate arguments to the measuring command, without quoting.
4. command — a program name with specified arguments.

## Supported workflows

### Measurement methods

In general any command printing a single numbers can be passed as a measuring method and plotted on a graph. Both `stdout` and `stderr` are collected together, so the resulting number can be in any of them. This is done to support the unusual behavior of the `time` command. 
There are two special measuring methods that get treated differently:
* `time` - measures the elapsed real time
* `perf` - captures events given by `perf-stat` (architecture dependant)

### Plotting options

Single value outputs (from custom or `time` measuring) are plotted on a linear graph, while `perf` is plotted over multiple graphs, one for each requested event.

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

## In-depth CLI

The benchmarker accepts following options:

* `-d`, `--db-path` — the path to the database location, default is `benchmarker.db` located in home directory. 

Subcommand should be provided after database:

* `run` — Executes, measures, and stores to the database the results of the measurements. It should be invoked from the inside of the repository holding the application being measured. After `run`, a benchmark named should be passed.
  * `-b`, `--benchmark-config-path` — the path to the configuration file of the benchmark
  * `-B`, `--backend-config-path` — the path to the configuration file of the backend
  * `-M`, `--benchmark-mode` — `used_cached`(default, uses data from database) or `force-rerun`(overrides database data).
  * The subcommand used profile should be passed next. We support the following:
    * time (default) — Measures lapsed  real  (wall clock) time used by the process, in seconds.
    * perf-stat — Gathers the performance counter statistics.
    * command - Custom measuring command. Should output exactly one number on either `stdout` or `stderr`.
    * flame-graph — Captures the data given by `perf record`, processes it and collapses the stack. It can be customized using the following options:
      * `-r`, `--flame-repo` — the path to the flame-graph repository of Brendan Gregg. 
      * `-f`, `--frequency` — the frequency at which it the run command be profiled.
      * `-s`, `--store-dir` — the directory in which to store the folded results.

* `plot` — Visualizes and compares the results of previous `runs`, reading them from the database
  After `plot` benchmark name should be passed. Plot type (subcommand) and its possible flags should be provided after the common options.
  * Common Options for `plot`
    * `-m`, `--measurement-method` — the command to measure the performance of the benchmark (e.g. time).
    * `-b`, `--benchmark-config-path` — the path to the configuration file of the benchmark
    * `-o`, `--output` — the path where plot should be saved
    * `--from <repository path:tag1,tag2,...>` — specifies which tags should be used in the comparison and to which repository they refer. Including this option multiple times adds more to the comparison. Here tags are used broadly, and include things like branches, tags, `HEAD`, with relative versions of thereof.
    * `-f`, `--format` — Output format of the rendered plot. Currently supported formats: `png` and `svg`.The output file extension **must match** the selected format to comply with the [plotters](https://docs.rs/plotters/latest/plotters/) API.
  * Options specific to a `series` plot
    * `-v`, `--visualization-kind` — Controls the style of the plot line. Can be `linear` for a standard line plot or `log` for a logarithmic plot. This affects the visual representation but does not rescale the underlying data.
  * Options specific to a `flamegraph` plot
    * Currently no additional flags are required.

* `database drop` or `database print` — prints or removes data from database.
  * `--commit-hash` Accepts list of accepted commit hashes divided by `:`, empty (or lack of argument) means it is not restricted.
  * `--benchmark-name` In same format restricts benchmark names.
  * `--measurement-method` In same format restricts measurement methods.       
  * `--benchmark-point` In same format restricts benchmark points. Provided elements must be non negative integers.

## Authors

* Paweł Mieszkowski
* Paweł Zalewski
* Krzysztof Hałubek
