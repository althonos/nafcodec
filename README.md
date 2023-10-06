# `naf.rs` [![Stars](https://img.shields.io/github/stars/althonos/naf-rs.svg?style=social&maxAge=3600&label=Star)](https://github.com/althonos/naf-rs/stargazers)

*Rust implementation of the [Nucleotide Archive Format (NAF)](https://github.com/KirillKryukov/naf)*.

[![Actions](https://img.shields.io/github/workflow/status/althonos/naf.rs/Test?style=flat-square&maxAge=600)](https://github.com/althonos/naf.rs/actions)
[![Codecov](https://img.shields.io/codecov/c/gh/althonos/naf.rs/master.svg?style=flat-square&maxAge=600)](https://codecov.io/gh/althonos/naf.rs)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square&maxAge=2678400)](https://choosealicense.com/licenses/mit/)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/naf.rs)
[![Crate](https://img.shields.io/crates/v/naf.svg?maxAge=600&style=flat-square)](https://crates.io/crates/naf)
[![Documentation](https://img.shields.io/badge/docs.rs-latest-4d76ae.svg?maxAge=2678400&style=flat-square)](https://docs.rs/naf)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/naf.rs/blob/master/CHANGELOG.md)
[![GitHub issues](https://img.shields.io/github/issues/althonos/naf.rs.svg?style=flat-square&maxAge=600)](https://github.com/althonos/naf.rs/issues)


## 🗺️ Overview

[Nucleotide Archive Format](https://github.com/KirillKryukov/naf) is a file 
format proposed in Kryukov *et al.*[\[1\]](#ref1) in 2019 for storing 
compressed nucleotide or protein sequences combining 4-bit encoding and 
[Zstandard](https://github.com/facebook/zstd) compression.

This crate provides a Rust implementation of a NAF decoder, from scratch, 
using `nom` for parsing the binary format, and `zstd-sys` for handling 
Zstandard decompression.

### 📋 Features

- **streaming decoder**: The decoder is implemented using different readers
  each accessing a region of the compressed file, allowing to stream records
  without having to decode full blocks.

The following features are planned:

- **optional decoding**: Allow the decoder to skip the decoding of certains 
  fields, such as ignoring quality strings when they are not needed.
- **encoder**: Implement an encoder as well, using either in-memory buffers
  or temporary files to grow the archive.

### 🔌 Usage

Use `naf::Decoder` to iterate over the contents of a Nucleotide Archive Format,
reading from any `Read` + `Seek` implementor:

```rust
let f = std::fs::File::open("data/LuxC.naf").unwrap();
let mut decoder = naf::decoder::Decoder::new(f).unwrap();

for result in decoder {
    let record = result.unwrap();
    // .. do something with the record .. //
}
```

All fields of the obtained `Record` are optional, and actually depend on the
kind of data that was compressed.

<!-- ## 🔍 See Also -->

## 💭 Feedback

### ⚠️ Issue Tracker

Found a bug ? Have an enhancement request ? Head over to the [GitHub issue
tracker](https://github.com/althonos/naf.rs/issues) if you need to report
or ask something. If you are filing in on a bug, please include as much
information as you can about the issue, and try to recreate the same bug
in a simple, easily reproducible situation.

<!-- ### 🏗️ Contributing

Contributions are more than welcome! See [`CONTRIBUTING.md`](https://github.com/althonos/naf.rs/blob/master/CONTRIBUTING.md) for more details. -->


## 📋 Changelog

This project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html)
and provides a [changelog](https://github.com/althonos/naf.rs/blob/master/CHANGELOG.md)
in the [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) format.

## ⚖️ License

This library is provided under the open-source
[MIT license](https://choosealicense.com/licenses/mit/). The NAF specification
is in the public domain.

*This project is in no way not affiliated, sponsored, or otherwise endorsed
by the [original NAF authors](https://github.com/KirillKryukov). It was 
developed by [Martin Larralde](https://github.com/althonos/) during his PhD 
project at the [European Molecular Biology Laboratory](https://www.embl.de/) 
in the [Zeller team](https://github.com/zellerlab).*

## 📚 References

- <a id="ref1">\[1\]</a> Kirill Kryukov, Mahoko Takahashi Ueda, So Nakagawa, Tadashi Imanishi. "Nucleotide Archival Format (NAF) enables efficient lossless reference-free compression of DNA sequences". Bioinformatics, Volume 35, Issue 19, October 2019, Pages 3826–3828. [doi:10.1093/bioinformatics/btz144](https://doi.org/10.1093/bioinformatics/btz144)