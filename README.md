# SDB — ScyllaDB Drivers Benchmarker

Ogólnie to dobry postęp w pracy - w połączeniu z otwartymi issue-ami powiedziałbym że zmierza w dobrą stronę.
Kod jest dość sensownie podzielony na moduły. Jeśli chodzi o funkcjonalność,
to robi dokładnie to co powinien - czyli mierzy zapisuje i plotuje wyniki,
chociaż z obecnym podejściem ciężko będzie dodać nowe rodzaje benchmarków -
kod jest trochę za mało modułowy w obszarach zbierania danych.

Ogólne uwagi do kodu:

- Dużo boiler plate kodu. Jest parę miejsc w kodzie, gdzie wprowadzamy dodatkowe kroki,
które nie dodają nic szczególnego, a zwiększają objętość kodu (Patrz: Executor::new i with_measure, albo add_argument vs with_arg).
Mam wrażenie, że to może być efekt mocnego użycia LLMÓw - dużo mniejszych funkcjonalności...

- Podobna kwestia z obsługą błędów. Niby dodaje trochę czytelności, ale kosztem dużego boilerplateu.
Rozbudowane błędy są istotne, by użytkownicy biblioteki mogli się matchować.
Jednak to narzędzie to binarka wykonywalna, nie wystawia API - tylko CLI.
Wobec tego ja bym szedł w stronę uproszczenia błędów (anyhow).

SDB is a tool for benchmarking, comparing, and visualizing the performance of different implementations.

## Quick Start

SDB has two main functions:

1. **`run`**: Execute a benchmark run and save the results.
2. **`plot`**: Visualize and compare results from previous runs.

Dobre readme powinno zawierać instrukcję o wszystkich potrzebnych zależnościach.

Mając ten błąd, czy jesteście w stanie powiedzieć, która biblioteka spowodowała ten problem (z tych co wy załączaliście),
i jak to naprawić? Jak to zrobicie, to dodajcie odpowiednie instrukcje do readme (np. kopiując to co jest w dokumentacji tamtej biblioteki)

error: failed to run custom build command for `yeslogic-fontconfig-sys v6.0.0`

Caused by:
  process didn't exit successfully: `/home/sczech/Documents/scylladb-drivers-benchmarker/target/debug/build/yeslogic-fontconfig-sys-d1f99327633ec853/build-script-build` (exit status: 101)
  --- stdout
  cargo:rerun-if-env-changed=RUST_FONTCONFIG_DLOPEN
  cargo:rerun-if-env-changed=FONTCONFIG_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_unknown_linux_gnu
  cargo:rerun-if-env-changed=HOST_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=FONTCONFIG_STATIC
  cargo:rerun-if-env-changed=FONTCONFIG_DYNAMIC
  cargo:rerun-if-env-changed=PKG_CONFIG_ALL_STATIC
  cargo:rerun-if-env-changed=PKG_CONFIG_ALL_DYNAMIC
  cargo:rerun-if-env-changed=PKG_CONFIG_PATH_x86_64-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_PATH_x86_64_unknown_linux_gnu
  cargo:rerun-if-env-changed=HOST_PKG_CONFIG_PATH
  cargo:rerun-if-env-changed=PKG_CONFIG_PATH
  cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR_x86_64-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR_x86_64_unknown_linux_gnu
  cargo:rerun-if-env-changed=HOST_PKG_CONFIG_LIBDIR
  cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-unknown-linux-gnu
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_unknown_linux_gnu
  cargo:rerun-if-env-changed=HOST_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR

  --- stderr

  thread 'main' panicked at /home/sczech/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/yeslogic-fontconfig-sys-6.0.0/build.rs:8:48:
  called `Result::unwrap()` on an `Err` value: "\npkg-config exited with status code 1\n> PKG_CONFIG_ALLOW_SYSTEM_LIBS=1 PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 pkg-config --libs --cflags fontconfig\n\nThe system library `fontconfig` required by crate `yeslogic-fontconfig-sys` was not found.\nThe file `fontconfig.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.\nThe PKG_CONFIG_PATH environment variable is not set.\n\nHINT: if you have installed the library, try setting PKG_CONFIG_PATH to the directory containing `fontconfig.pc`.\n"
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...

This also includes dependencies for integration tests...

test test_cpp_vs_rust ... FAILED

failures:

---- test_cpp_vs_rust stdout ----
./tests/data/cpp/
./tests/data/rust/
my_stdout:
my_stderr: Error: Benchmarking(Compile(CompilationRunning { output: Output { status: ExitStatus(unix_wait_status(512)), stdout: "g++ -Ofast -static -march=native -ftree-vectorize    regex.cpp   -o regex\n", stderr: "/usr/bin/ld: cannot find -lstdc++: No such file or directory\n/usr/bin/ld: have you installed the static version of the stdc++ library ?\n/usr/bin/ld: cannot find -lm: No such file or directory\n/usr/bin/ld: have you installed the static version of the m library ?\n/usr/bin/ld: cannot find -lc: No such file or directory\n/usr/bin/ld: have you installed the static version of the c library ?\ncollect2: error: ld returned 1 exit status\nmake: *** [<builtin>: regex] Error 1\n" } }))

./tests/data/cpp/

thread 'test_cpp_vs_rust' panicked at tests/integration_test.rs:70:9:
assertion failed: output.status.success()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    test_cpp_vs_rust

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s

### Example usage

From inside the `tests/data/cpp` and `tests/data/rust` directories execute:

```sh
cargo run regex -d ../test.db -b ../config.yml run
```

Then, later to graph the results execute (from `tests/data`):

```sh
cargo run regex -b config.yml -d test.db plot --from=cpp/:HEAD --from=rust/:HEAD
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

Ah tak, artefakty LLMów. Gdyby jeszcze nie psuły poprawności dokumentacji...
<https://medium.com/@brentcsutoras/the-em-dash-dilemma-how-a-punctuation-mark-became-ais-stubborn-signature-684fbcc9f559>

## Authors

* Paweł Mieszkowski
* Paweł Zalewski
* Krzysztof Hałubek
