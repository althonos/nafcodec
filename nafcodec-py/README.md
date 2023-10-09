# 📦🧬 `nafcodec` [![Stars](https://img.shields.io/github/stars/althonos/nafcodec.svg?style=social&maxAge=3600&label=Star)](https://github.com/althonos/nafcodec/stargazers)

*Rust coder/decoder for [Nucleotide Archive Format (NAF)](https://github.com/KirillKryukov/naf) files*.

[![Actions](https://img.shields.io/github/actions/workflow/status/althonos/nafcodec/python.yml?branch=main&logo=github&style=flat-square&maxAge=300)](https://github.com/althonos/nafcodec/actions)
[![Coverage](https://img.shields.io/codecov/c/gh/althonos/nafcodec?logo=codecov&style=flat-square&maxAge=3600)](https://app.codecov.io/gh/althonos/nafcodec)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square&maxAge=2678400)](https://choosealicense.com/licenses/mit/)
[![Docs](https://img.shields.io/readthedocs/nafcodec/latest?style=flat-square&maxAge=600)](https://nafcodec.readthedocs.io)
[![Crate](https://img.shields.io/crates/v/nafcodec-py.svg?maxAge=600&style=flat-square)](https://crates.io/crates/nafcodec-py)
[![PyPI](https://img.shields.io/pypi/v/nafcodec.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nafcodec)
[![Wheel](https://img.shields.io/pypi/wheel/nafcodec.svg?style=flat-square&maxAge=2678400)](https://pypi.org/project/nafcodec/#files)
[![Bioconda](https://img.shields.io/conda/vn/bioconda/nafcodec?style=flat-square&maxAge=3600)](https://anaconda.org/bioconda/nafcodec)
[![Python Versions](https://img.shields.io/pypi/pyversions/nafcodec.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nafcodec/#files)
[![Python Implementations](https://img.shields.io/pypi/implementation/nafcodec.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nafcodec/#files)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/nafcodec/tree/main/nafcodec-py)
[![Mirror](https://img.shields.io/badge/mirror-EMBL-009f4d?style=flat-square&maxAge=2678400)](https://git.embl.de/larralde/nafcodec/)
[![GitHub issues](https://img.shields.io/github/issues/althonos/nafcodec.svg?style=flat-square&maxAge=600)](https://github.com/althonos/nafcodec/issues)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/nafcodec/blob/master/CHANGELOG.md)
[![Downloads](https://img.shields.io/pypi/dm/nafcodec?style=flat-square&color=303f9f&maxAge=86400&label=downloads)](https://pepy.tech/project/nafcodec)


## 🗺️ Overview

[Nucleotide Archive Format](https://github.com/KirillKryukov/naf) is a file
format proposed in Kryukov *et al.*[\[1\]](#ref1) in 2019 for storing
compressed nucleotide or protein sequences combining 4-bit encoding and
[Zstandard](https://github.com/facebook/zstd) compression. NAF files can
be compressed and decompressed using the
[original C implementation](https://kirill-kryukov.com/study/naf).

This library provides [PyO3](https://pyo3.rs) bindings to the `nafcodec` crate,
a Rust implementation of a NAF decoder using [`nom`](https://crates.io/crates/nom) 
for parsing the binary format, and [`zstd`](https://crates.io/crates/zstd) for 
handling Zstandard decompression. It provides a complete API that allows 
iterating over the contents of a NAF file.

*This is the Python version, there is a [Rust crate](https://crates.io/crates/nafcodec) available as well.*

### 📋 Features

- **streaming decoder**: The decoder is implemented using different readers
  each accessing a region of the compressed file, allowing to stream records
  without having to decode full blocks.
- **file-like decoding**: Allow the decoder to read from a file-like object
  instead of expecting a path.

The following features are planned:

- **optional decoding**: Allow the decoder to skip the decoding of certains
  fields, such as ignoring quality strings when they are not needed.
- **encoder**: Implement an encoder as well, using either in-memory buffers
  or temporary files to grow the archive.

### 🔌 Usage

Use a `nafcodec.Decoder` to iterate over the contents of a Nucleotide Archive 
Format, reading from the given [path-like](https://docs.python.org/3/glossary.html#term-path-like-object)
or [file-like](https://docs.python.org/3/glossary.html#term-file-object) object:

```python
import nafcodec

decoder = nafcodec.Decoder("../data/LuxC.naf")
for record in decoder:
    print(record.id)
```

All fields of the obtained `Record` are optional, and actually depend on the
kind of data that was compressed. 

<!-- ## 🔍 See Also -->

## 💭 Feedback

### ⚠️ Issue Tracker

Found a bug ? Have an enhancement request ? Head over to the [GitHub issue
tracker](https://github.com/althonos/nafcodec/issues) if you need to report
or ask something. If you are filing in on a bug, please include as much
information as you can about the issue, and try to recreate the same bug
in a simple, easily reproducible situation.

<!-- ### 🏗️ Contributing

Contributions are more than welcome! See [`CONTRIBUTING.md`](https://github.com/althonos/nafcodec/blob/master/CONTRIBUTING.md) for more details. -->


## 📋 Changelog

This project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html)
and provides a [changelog](https://github.com/althonos/nafcodec/blob/master/CHANGELOG.md)
in the [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) format.

## ⚖️ License

This library is provided under the open-source
[MIT license](https://choosealicense.com/licenses/mit/). The
[NAF specification](https://github.com/KirillKryukov/naf/blob/master/NAFv2.pdf)
is in the public domain.

*This project is in no way not affiliated, sponsored, or otherwise endorsed
by the [original NAF authors](https://github.com/KirillKryukov). It was
developed by [Martin Larralde](https://github.com/althonos/) during his PhD
project at the [European Molecular Biology Laboratory](https://www.embl.de/)
in the [Zeller team](https://github.com/zellerlab).*

## 📚 References

- <a id="ref1">\[1\]</a> Kirill Kryukov, Mahoko Takahashi Ueda, So Nakagawa, Tadashi Imanishi. "Nucleotide Archival Format (NAF) enables efficient lossless reference-free compression of DNA sequences". Bioinformatics, Volume 35, Issue 19, October 2019, Pages 3826–3828. [doi:10.1093/bioinformatics/btz144](https://doi.org/10.1093/bioinformatics/btz144)
