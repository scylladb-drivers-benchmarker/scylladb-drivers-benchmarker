# SDB — ScyllaDB Drivers Benchmarker

SDB is a tool for benchmarking, comparing, and visualizing the performance of different implementations.

## Quick Start

SDB has two main functions:

1. **`run`**: Execute a benchmark run and save the results.
2. **`plot`**: Visualize and compare results from previous runs.

### Example usage

From inside the `tests/data/cpp` directory execute:

```sh
cargo run regex -d ../ -b ../config.yml run
```

Then, later:

```sh
cargo run regex -b ../config.yml -d ../test.db plot --from=.:HEAD
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

* `-d`, `—db-path` — the path to the database location
* `-b`, `—benchmark-config-path` — the path to the configuration file of the benchmark
* `-m`, `—measurement-method` — the command to measure the performance of the benchmark (e.g. time).

The benchmark name should be passed next and be followed by one of the two subcommands:

* `run` — Executes, measures, and stores to the database the results of the measurements. It should be invoked from the inside of the repository holding the application being measured.
  * `-b`, `—backend-config-path` — the path to the configuration file of the backend
* `plot` — Visualizes and compares the results of previous `runs`, reading them from the database
  * `--from <repository path:tag1,tag2,...>` — specifies which tags should be used in the comparison and to which repository they refer. Including this option multiple times adds more to the comparison. Here tags are used broadly, and include things like branches, tags, `HEAD`, with relative versions of thereof.

## Authors

* Paweł Mieszkowski
* Paweł Zalewski
* Krzysztof Hałubek
