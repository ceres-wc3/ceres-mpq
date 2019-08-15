# About

`ceres-mpq` is a pure-Rust implementation of a reader/writer for Blizzard's proprietary MoPaQ (MPQ) archive format.

The intended use-case for this library is to be used for reading and writing Warcraft III map files, which in themselves are MPQ archives. Since Warcraft III only uses Version 1 of the format, no effort is made here to support newer features found in MPQ files in other games.

For more details and the list of supported/unsupported features, please refer to the top-level library [documentation](https://docs.rs/ceres-mpq).

# Command-line

There is a command-line utility for reading, viewing, and writing MPQ files, which you can find here: [ceres-mpqtool](https://github.com/ElusiveMori/ceres-mpqtool)