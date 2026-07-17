# gacs

A deterministic ASCII character generator written in Rust.

`gacs` is a CLI tool that generates reproducible ASCII strings (such as passwords or secret tokens) based on a seed string, an optional file-based salt, and customizable character sets. It leverages cryptographically secure hashing (e.g., SHA-512) to ensure the outputs are unpredictable without the exact key parameters.

## Features

* **Deterministic Generation**: Generates the exact same character sequence every time, provided the same seed and parameters are used.
* **File-Based Salting**: Seamlessly incorporates any local file (images, documents, audio) as a salt. Large files are streamed efficiently with minimal memory overhead.
* **Flexible Pre-defined Character Sets**: Offers 4 built-in base sets optimized for different use cases:
  * Standard BASE64 set.
  * URL-Safe characters.
  * Password-Safe characters (excludes visually ambiguous characters like `O`, `0`, `l`, `1` and introduces symbols).
  * Shell-Safe characters (includes only alphanumeric characters, `.`, and `_` to avoid shell-specific quotation issues).
* **Custom Character Modification Rules**: Allows you to temporarily remove specific characters from the base set and append new ones to meet specific string policies.

## Installation & Build

### Pre-built Binaries

For a quick setup without a Rust environment, download the pre-compiled binaries for your platform from the [GitHub Releases](https://github.com/comosense/gacs/releases) page.

We provide lightweight, size-optimized binaries for the following targets:

| OS | Architecture / Environment | Archive File |
| :--- | :--- | :--- |
| **Linux (GNU)** | x86_64 | `gacs-x86_64-unknown-linux-gnu.tar.gz` |
| | AArch64 | `gacs-aarch64-unknown-linux-gnu.tar.gz` |
| **Linux (musl)** | x86_64 | `gacs-x86_64-unknown-linux-musl.tar.gz` |
| | AArch64 | `gacs-aarch64-unknown-linux-musl.tar.gz` |
| **macOS** | Apple Silicon (AArch64) | `gacs-aarch64-apple-darwin.tar.gz` |
| | Intel (x86_64) | `gacs-x86_64-apple-darwin.tar.gz` |
| **Windows** | x86_64 (MSVC) | `gacs-x86_64-pc-windows-msvc.zip` |
| | AArch64 (MSVC) | `gacs-aarch64-pc-windows-msvc.zip` |

Extract the archive and move the `gacs` (or `gacs.exe`) binary to a directory in your system's `PATH`.

[!NOTE]
**Which Linux binary should I choose?**

* **GNU (`-gnu`)**: Dynamically linked against `glibc`. This is the standard choice for most mainstream Linux distributions (Ubuntu, Debian, Fedora, Arch, etc.).

* **musl (`-musl`)**: Statically linked against `musl-libc`. Although the binary size is slightly larger, it has zero external dependencies and runs seamlessly on minimal environments like Alpine Linux, OpenWrt, or distroless Docker containers.

---

### Building from Source

If you prefer to compile `gacs` manually, you will need a Rust development environment installed (`cargo`). Clone the repository and build the binary:

```bash
git clone https://github.com/comosense/gacs.git
cd gacs

# Build with release optimizations
cargo build --release

# The compiled binary will be located at:
./target/release/gacs --help

```

## Usage

### Basic Generation

Provide a seed string to generate a deterministic 32-character string using the default Password-Safe (`ps`) character set.

```bash
$ gacs my_secret_seed
@GP5m7ijz@R@yXoasokyE86PjTqWeYMc

```

*If the seed argument is omitted, a random seed will be automatically generated based on the system time.*

### Detailed Output (Verbose Mode)

Use the `-v` (`--verbose`) flag to display the generated output alongside the exact parameters and the finalized character table used.

```bash
$ gacs my_secret_seed -v
@GP5m7ijz@R@yXoasokyE86PjTqWeYMc
  [SEED] my_secret_seed
  [LENGTH] 32
  [CHARSET] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```

### Adjusting Length and Character Sets

Modify the output length with `-l` (`--length`) and switch the character set via `-c` (`--charset`).

* `64` (BASE64): `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/`
* `us` (URL-Safe): `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_`
* `ps` (Password-Safe): `ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_`
* `ss` (Shell-Safe): `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._`

```bash
# Generate a 16-character URL-Safe string
$ gacs my_secret_seed -l 16 -c us
OGP5m7ijzOROyXoa

```

### Custom Character Modification Rules

Modify the character set by removing specific characters and appending new ones using the `-r` (`--rule`) flag. Format: `'characters_to_remove:characters_to_add'`.

```bash
# Remove 'Z', 'z', and '9' from the charset, and append '^', '&', and '*'
$ gacs my_secret_seed -r 'Zz9:^&*'
@GP7n-jk%@R@$Xpbtp#$E_8PkTrWfYMd

```

### File Salting

Use the `-s` (`--salt`) option to introduce a local file as an additional layer of entropy. The output will only match if both the seed and the file contents are identical.

```bash
# Salt the generation with a local image file
$ gacs my_secret_seed -s path/to/secret_image.jpg

```

### Bulk Generation

Generate multiple independent strings at once using the `-N` (`--number`) flag. Note that when generating multiple strings, seeds are automatically generated; this option cannot be used simultaneously with a manual `[SEED]` argument.
To view or save the auto-generated seeds for future reproduction, combine this option with the `-v` (`--verbose`) flag.

```bash
# Generate 3 strings simultaneously with their auto-generated seeds
$ gacs -N 3 -v
sF@#%Rbq5CM%Y9CxL%6Xpo3$okq$RnJT
  [SEED(Auto)] q1akBXyCPWB8vneEn3UKkCA9vqN7AI8MYIRH0gPfDangyi3DqoUh8.CJlggUDSv3XBCiYffVle9w_Grx
r4$m#HGvP-_ysF35xH2s7EC5Jq#xaiNb
  [SEED(Auto)] Xi9uC74BV0dQ2ys0J3iCyDSxUASZGRdW8sYeR7zDkHYPHM8KuosaSOtyMXKgN69PbfHjtTzS7knT8SWL
jT_V@beU-KU5%Li#G_hRWDPVLvK2$FAq
  [SEED(Auto)] o2.faPVAuJsR1Zxuz73TaLdXNcXCtFkw7LAjU9IixkshYNcclvWcSYHAekBUXJbh8gwCa.hpyowIMI1J
  [LENGTH] 32
  [CHARSET] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```

### Controlling Auto-Generated Seed Length

When you omit the manual `[SEED]` argument (or use `-N` for bulk generation), `gacs` automatically generates seed strings. By default, it generates them at the maximum possible length.
You can customize the length of these auto-generated seeds using the `-L` (`--slength`) option.

```bash
# Generate a string using a shorter 16-character auto-generated seed
$ gacs -L 16 -v
WPkMAdVNBhJ2$fG24ZWZUVN#Ss-qR$gZ
  [SEED(Auto)] rcJsnGlKN6KvBU1W
  [LENGTH] 32
  [CHARSET] ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_

```

[!IMPORTANT]
The `-L` (`--slength`) option specifically configures the length of the seed, not the final output string. It conflicts with a manual `[SEED]` argument and can only be used when seeds are being auto-generated.

## Command Line Options

```text
Arguments:
  [SEED]  Base string to generate the characters from (generated automatically if omitted)

Options:
  -s, --salt <FILE>         Optional file to use as an additional cryptographic salt
  -l, --length <LENGTH>     Length of the generated characters [default: 32]
  -c, --charset <CHARSET>   Character set to use (64, us, ps, ss) [default: ps]
  -r, --rule <RULE>         Modify the charset by removing and appending characters (Format: 'remove:add')
  -N, --number <NUMBER>     Number of strings to generate
                            When this is set, seeds are auto-generated; conflicts with [SEED]
  -L, --slength <NUMBER>    Length of the seed to auto-generate; conflicts with [SEED]
                            If omitted, the maximum possible length is applied
  -v, --verbose             Print detailed configuration along with the generated characters
  -h, --help                Print help
  -V, --version             Print version

```

## Security Considerations

* **Determinism**: Since gacs is a deterministic generator, the security of the output sequence relies entirely on the secrecy and entropy of your `[SEED]` and the optional salt file. *(Avoid using easily guessable seeds.)*

* **Algorithm**: gacs uses SHA-512 to deterministically expand the seed, ensuring that even a 1-bit change in the seed or salt file results in a completely different output (avalanche effect).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
