# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).


## [Unreleased]
[Unreleased]: https://github.com/althonos/uniprot.rs/compare/v0.3.0...HEAD


## [v0.3.0] - 2024-01-31
[v0.3.0]: https://github.com/althonos/uniprot.rs/compare/v0.2.0...v0.3.0

### `nafcodec`

#### Changed
- Update `nom` to `v8.0`.

#### Fixed
- `Encoder::push` potentially recording length twice when given both sequence and quality string.
- `SequenceWriter::write` not correctly reporting the number of bytes written for a sequence ([#3](https://github.com/althonos/nafcodec/issues/3)).


### `nafcodec-py`

#### Changed
- Update `pyo3` to `v0.23`.
- Drop compilation of `tempfile` in `nafcodec-py` wrapper.
- Make fields of `Record` struct copy-on-write.
- Avoid copying data from Python heap in `Encoder`.
- Use `maturin` instead of `setuptools-rust` to build the Python wheels.

#### Fixed
- Use `std::os::raw::c_char` to fix Aarch64 build ([#2](https://github.com/althonos/nafcodec/issues/2)).


## [v0.2.0] - 2024-04-10
[v0.2.0]: https://github.com/althonos/uniprot.rs/compare/v0.1.1...v0.2.0

### `nafcodec`

#### Added
- `Encoder` to write a Nucleotide Archive Format file using generic temporary storage.
- `Flag` enum to simplify creation and manipulation of `Flags` objects without `u8` arithmetics.
- Builder types for `Encoder` and `Decoder` creation with different configurations.
- `ExactSizeDecoder` trait implementation to `Decoder`.
- Error variants to return on missing fields and invalid sequence characters.

#### Changed
- Use iterators internally inside reader code.
- Bumped `pyo3` dependency to `v0.21.1`.
- Bumped `zstd` dependency to `v0.13.1`.

#### Fixed
- Decoding of record lengths when `quality` field is requested without `sequence`.
- Compilation issue on 32-bit platforms in parser code.

### `nafcodec-py`

#### Added
- Support for file-like objects in `Decoder`.
- Proper error conversion for `seek` failures.
- `Decoder.sequence_type` property to get the type of sequence being decoded.
- Constructor for `Record` objects.
- `Decoder` properties to access decoded header data.
- `Decoder.__len__` implementation using number of sequences in archive.
- `Encoder` class to support encoding `Record` objects into an archive.
- `nafcodec.open` function to get a decoder or encoder based on literal mode.
- Context manager protocol implementation to `Decoder` objects.

#### Changed
- Use `readinto` method of file-like objects given to `Decoder` if available.


## [v0.1.1] - 2023-10-08
[v0.1.1]: https://github.com/althonos/uniprot.rs/compare/v0.1.0...v0.1.1

### Added
- `nafcodec-py` crate with PyO3 bindings.


## [v0.1.0] - 2023-10-07
[v0.1.0]: https://github.com/althonos/uniprot.rs/compare/1923371...v0.1.0

Initial release.
