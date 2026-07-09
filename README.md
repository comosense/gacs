# gacs

A deterministic ASCII character generator written in Rust.

`gacs` is a CLI tool that generates reproducible ASCII strings (such as strong passwords or secret tokens) based on a seed string, an optional file-based salt, and customizable character sets.

## Features

* **Deterministic Generation**: Generates the exact same character sequence every time, provided the same seed and parameters are used.
* **File-Based Salting**: Seamlessly incorporates any local file (images, documents, audio) as a salt. Large files are streamed efficiently with minimal memory overhead.
* **Flexible Pre-defined Character Sets**: Offers 3 built-in base sets optimized for different use cases:
  * `64`: Standard BASE64 set.
  * `us`: URL-Safe characters.
  * `ps`: Password-Safe characters (excludes visually ambiguous characters like `O`, `0`, `l`, `1` and introduces symbols).
* **Custom Character Modification Rules**: Allows you to temporarily remove specific characters from the base set and append new ones to meet specific string policies.

## Installation

Ensure you have a Rust development environment installed (`cargo`). Clone the repository and build the binary:

```bash
git clone https://github.com/comosense/gacs.git
cd gacs

# Build with release optimizations
cargo build --release

# The compiled binary will be located at:
./target/release/gacs --help

```

> *To minimize the final binary size, it is highly recommended to enable Link Time Optimization (LTO) and strip symbols in your `Cargo.toml`.*

## Usage

### Basic Generation

Provide a seed string to generate a deterministic 32-character string using the default Password-Safe (`ps`) character set.

```bash
$ gacs my_secret_seed
@GP5m7ijz@...

```

> *If the seed argument is omitted, a random seed will be automatically generated based on the system time.*

### Detailed Output (Verbose Mode)

Use the `-v` (`--verbose`) flag to display the generated output alongside the exact parameters and the finalized character table used.

```bash
$ gacs my_secret_seed -v
@GP5m7ijz@...
  [SEED] my_secret_seed
  [LENGTH] 32
  [CHARSET] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```

### Adjusting Length and Character Sets

Modify the output length with `-l` (`--length`) and switch the character set via `-c` (`--charset`) (64, us, ps).

* `64` (BASE64): ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/
* `us` (URL-Safe): ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_
* `ps` (Password-Safe): ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```bash
# Generate a 16-character URL-Safe string
$ gacs my_secret_seed -l 16 -c us
OGP5m7ijzO...

```

### Custom Character Modification Rules

Modify the character set by removing specific characters and appending new ones using the `-r` (`--rule`) flag. Format: `'characters_to_remove:characters_to_add'`.

```bash
# Remove 'Z', 'z', and '9' from the charset, and append '^', '&', and '*'
$ gacs my_secret_seed -r 'Zz9:^&*'
@GP7n-jk%@...

```

### File Salting

Use the `-s` (`--salt`) option to introduce a local file as an additional layer of entropy. The output will only match if both the seed and the file contents are identical.

```bash
# Salt the generation with a local image file
$ gacs my_secret_seed -s path/to/secret_image.jpg

```

### Bulk Generation

Generate multiple independent strings at once using the `-n` (`--number`) flag. Note that when generating multiple strings, individual seeds are automatically managed.

```bash
# Generate 5 strings simultaneously
$ gacs -n 5

```

## Command Line Options

```text
Arguments:
  [SEED]  Base string to generate the characters from (generated automatically if omitted)

Options:
  -s, --salt <FILE>         Optional file to use as an additional cryptographic salt
  -l, --length <LENGTH>     Length of the generated characters [default: 32]
                            Setting this to 0 generates the maximum possible length
  -c, --charset <CHARSET>   Character set to use (64, us, ps) [default: ps]
  -r, --rule <RULE>         Modify the charset by removing and appending characters (Format: 'remove:add')
  -n, --number <NUMBER>     Number of strings to generate
                            Setting this, seeds are auto-generated; conflicts with [SEED]
  -v, --verbose             Print detailed configuration along with the generated characters
  -h, --help                Print help
  -V, --version             Print version

```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
