# Contribution guidelines

First off, thank you for considering contributing to xml-disassembler.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## Reporting issues

Before reporting an issue on the
[issue tracker](https://github.com/mcarvin8/xml-disassembler-rust/issues),
please check that it has not already been reported by searching for some related
keywords.

## Pull requests

Try to do one pull request per change.

### Releasing

Releases and changelog generation is fully automated using release-plz.

To ensure your changes are properly categorized in the changelog, please follow [conventional commit messages](https://www.conventionalcommits.org/en/v1.0.0/).

### CI & Code Coverage

All pull requests run automated CI checks. Tests are executed using cargo-llvm-cov and coverage reports are uploaded to Codecov. PRs must pass all tests and hit 90% coverage before merging.

You can optionally run this command locally to run tests and generate coverage:

```bash
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'
```

## Developing

### Set up

This is no different than other Rust projects.

```shell
git clone https://github.com/mcarvin8/xml-disassembler-rust
cd xml-disassembler
cargo build
```
## Testing

Run all tests:

```bash
cargo test
```

- **Unit tests** – In-module tests for parsers, builders, and merge logic (e.g. `strip_whitespace`, `merge_xml_elements`, `extract_root_attributes`, `parse_xml`).
- **Integration test** – `tests/disassemble_reassemble.rs` runs a full round-trip: disassemble a fixture XML, reassemble it, and assert the reassembled content equals the original file.

### Useful Commands

- Build and run release version:

  ```shell
  cargo build --release && cargo run --release
  ```

- Run Clippy:

  ```shell
  cargo clippy --all-targets --all-features --workspace
  ```

- Run all tests:

  ```shell
  cargo test --all-features --workspace
  ```

- Run all tests with code coverage (install llvm-cov first):

  ```shell
  cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info --ignore-filename-regex 'main\.rs'
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt --all -- --check
  ```

- Format the code in the project

  ```shell
  cargo fmt --all
  ```
