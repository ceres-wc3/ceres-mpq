# About

`ceres-mpq` is a pure-Rust implementation of a reader/writer for Blizzard's proprietary MoPaQ (MPQ) archive format.

The intended use-case for this library is to be used for reading and writing Warcraft III map files, which in themselves are MPQ archives. Since Warcraft III only uses Version 1 of the format, no effort is made here to support newer features found in MPQ files in other games.

For more details and the list of supported/unsupported features, please refer to the top-level library [documentation](https://docs.rs/ceres-mpq).

# Command-line

This repository also contains a command-line interface to the library, `mpqtool`. Currently, it is WIP.