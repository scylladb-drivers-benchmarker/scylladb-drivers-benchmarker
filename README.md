# SDB

**ScyllaDB Drivers Benchmarker** is a tool for comparing and visualizing the performance of applications.

We assume each compared application is held in a different git repository, or at a different commit then another. This assumption lets us identify solutions by their commit hashes.

Benchmarking results are stored in a local database.

## In-depth CLI

The benchmarker accepts following options:

* -d, --db-path -- the path to the database location
* -b, --benchmark-config-path -- the path to the configuration file of the benchmark
* -m, --measurement-method -- the command to measure the performance of the benchmark (eg. time). The command used to run the benchmark and all of it's arguments will be passed as separate argument to the measuring command, without quoting.

The benchmark name should be passed next, and be followed by one of the two subcommands:

* run -- Executes, measures and stores to the database the results of the measurements. It should be invoked from the inside of the repository holding the application being measured.
* plot -- Visualizes and compares the results of previous `runs`, reading them from the database
  * --from=\<repository path\>:tags,... -- specifies which tags should be used in the comparison and to which repository they refer to. Including this option multiple times adds more to the comparison. Here tags are used broadly, and include things like branches, tags, `HEAD`, with relative versions of thereof.

## Example

### Running the benchmarks

```sh
sdb "select" run
```

## Plotting the results

```sh
sdb "select" plot --from=nodejs-driver:v1.1,v1.2 --from=python-driver:main,main~10
```

## Authors

* Paweł Mieszkowski
* Paweł Zalewski
* Krzysztof Hałubek
