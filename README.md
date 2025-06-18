# vevolabparser

vevolabparser is a Rust package for parsing output from FUJIFILM VisualSonics Vevo
LAB (v5.9.0, build 3539, Report Version 8).

This parser is designed to extract structured data from echocardiogram reports that
reference the following sxml:

```raw
Measurement File: VSI_CardiacPackage.sxml
Measurement Version: 5
Measurement Description: Cardiac Package
```

## Table of Contents
- [Installation](#installation)
    - [From crates](#from-crates)
    - [From Release Binaries](#from-release-binaries)
- [Output](#output)

## Installation

### From crates (recommended)

First, [install Rust](https://www.rust-lang.org/tools/install). This includes the
`cargo` package manager, which is used to install Rust packages.

Then you can install vevolabparser with the following command:

```bash
cargo install vevolabparser
```

This will make the `vevolabparser` command available in your `$PATH`, allowing you to
run it from any terminal, e.g.:

```bash
vevolabparser --help
```

### From Release Binaries

**Note:** This does not require that you install Rust -- you can just use the binaries
without any other setup.

Precompiled binaries for Ubuntu, macOS, and Windows are available on the
GitHub Releases page.

For example, if you are on macOS, download the file named:

`vevolabparser-macos-latest`

You may also want to rename it to something simpler (`mv` is the cmd you'd use
from the terminal. You can also re-name it in your file explorer):

```bash
mv vevolabparser-macos-latest vevolabparser
```

You can then run it like any other CLI tool:

```bash
./vevolabparser --help
```

For Windows, the file will be called:

```bash
vevolabparser-windows-latest.exe
```

You can run it from cmd or PowerShell directly. If you are not using the terminal,
consult online resources for how to make .exe files easily accessible from your
file system.

## Output

You can run this with example data by downloading the
[example CSV file](https://github.com/cmatKhan/vevolabparser/blob/main/test_data/2025-06-04-15-49-24.csv)

And then execute the parser like so (note that `vevolabparser` needs to be either a
path to the binary, or in your `$PATH`):

```bash
vevolabparser 2025-06-04-15-49-24.csv
```

This will output two files:

### `measurement_<input_filename>>.csv` 

This contains the measurement tables for each Sample and Protocol. It has the
following columns:

- `id`: This is the `Series Name` from the input csv file
- `protocol`: This is the `Protocol Name` from the input csv file,
ie 'MV Flow' or 'SAX M-Mode'
  
And the following columns, which are directly from the input file. See the sxml or 
vevolab documentation for more details:

- `measurement`
- `mode`
- `parameter`
- `units`
- `avg`
- `std`
- `instance_1`
- `instance_2`

### `calculations_<input_filename>>.csv`

This contains the calculation tables for each Sample and Protocol. It has the
following columns:

- `id`: This is the `Series Name` from the input csv file
- `protocol`: This is the `Protocol Name` from the input csv file,
ie 'MV Flow' or 'SAX M-Mode'

And the following columns, which are directly from the input file. See the sxml or 
vevolab documentation for more details:

- `calculation`
- `units`
- `value`