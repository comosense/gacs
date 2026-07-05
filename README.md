# gacs

A deterministic ASCII character generator written in Rust.

`gacs` is a fast CLI tool that generates reproducible ASCII characters based on a given seed string, an optional salt (file), and specific character sets.

## Features

* **Deterministic Generation**: Generate the exact same ASCII characters (such as a strong password) every time, as long as you provide the same seed and conditions.
* **File-based Salting**: Use any local file (images, documents, etc.) as a cryptographic salt. The tool streams large files efficiently without consuming massive amounts of memory.
* **Flexible Character Sets**: Choose from 3 built-in base character sets depending on your needs:
  * `64`: BASE64
  * `us`: URL safe
  * `ps`: Password safe (removes visually confusing characters and adds symbols)
* **Custom Replacement Rules**: Flexibly define rules to replace specific characters in the base set (e.g., replacing `O` and `0` with `@`).
* **Fast & Lightweight**: Achieves exceptionally high performance with minimal memory allocation, leveraging Rust's zero-cost abstractions.

## Installation

You will need a Rust build environment (`cargo`). Clone the repository and build it locally.

```bash
git clone https://github.com/comosense/gacs.git
cd gacs

# Build with optimizations
cargo build --release

# The compiled binary will be located in the target/release/ directory
./target/release/gacs --help

```

> *(If you want to minimize the compiled binary size, it is highly recommended to add LTO (Link Time Optimization) and strip settings to your `Cargo.toml` before building.)*

## Usage

### Basic Generation

Pass a seed string to generate characters using the default settings (32 characters, password-safe charset).

```bash
$ gacs my_secret_seed
pgHuENPMgR...

```

> *(If the seed is omitted, the tool automatically generates a seed based on the current system time.)*

### Detailed Output Mode

Add the `-v` (`--verbose`) flag to display the exact parameters (seed, charset, etc.) used during generation alongside the final characters.

```bash
$ gacs my_secret_seed -v
pgHuENPMgR...
 [SEED] my_secret_seed
 [LENGTH] 32
 [CHARSET] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```

### Changing Length and Charset

Use `-l` to specify the length and `-c` to choose the character set (`64`, `us`, `ps`).

* [`64`: BASE64] ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/
* [`us`: URL safe] ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_
* [`ps`: Password safe] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```bash
# Generate a 16-character URL-safe string
$ gacs my_secret_seed -l 16 -c us

```

### Using a File as a Salt

Use `-s` to incorporate a local file (such as a secret image or document) as an additional input.

```bash
# Using an image file as a salt
$ gacs my_secret_seed -s path/to/secret_image.jpg

```

### Applying Custom Replacement Rules

Use `-r` to replace specific characters in the chosen charset. The format is `target_characters:replacement_characters`.

```bash
# Replace 'Z', 'z', and '9' with '^', '&', and '*'
$ gacs my_secret_seed -r 'Zz9:^&*'

```

## Options

```text
Arguments:
  [SEED]  Base string to generate the characters from

Options:
  -c, --charset <CHARSET>   Character set to use (64,us,ps) [default: ps]
  -s, --salt <FILE>         Optional file to use as an additional cryptographic salt
  -l, --length <LENGTH>     Length of the generated characters [default: 32]
  -r, --rule <RULE>         Replace specific characters in the charset (Format: 'target:replacement')
  -v, --verbose             Print detailed configuration along with the generated characters
  -h, --help                Print help
  -V, --version             Print version

```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
