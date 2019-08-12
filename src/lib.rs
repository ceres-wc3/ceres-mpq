//! A library for reading and writing Blizzard's proprietary MoPaQ archive format.
//! 
//! Currently, `ceres-mpq` only supports reading and writing Version 1 MoPaQ
//! archives, as this is the only version of the format still actively encountered 
//! in the wild, used by Warcraft III custom maps.
//! 
//! For this reason, no effort was made to support features found in newer
//! versions of the format, though this may change in the future if there is
//! a need for this.
//! 
//! `ceres-mpq` provides no support to edit existing archives yet, thought it may in the future.
//!
//! # Supported features
//! 
//! Not the whole range of MPQ features is supported yet for reading archives. Notably:
//! 
//! * IMA ADPCM compression is unsupported. This is usually present on `.wav` files.
//! * Huffman coding compression is unsupported. This is usually present on `.wav` files.
//! * PKWare DCL compression is unsupported. However, I haven't seen any WC3 maps that use it.
//! * Single-unit files are unsupported.
//! * Checksums and file attributes are not checked or read.
//! 
//! Additionally, for writing archives:
//! * You cannot choose which compression type to use for added files in [Creator](struct.Creator.html). DEFLATE is used by default.
//! 
//! # Protected MPQs
//! 
//! In Warcraft III, it is not uncommon to encounter so-called "protected maps" which use various
//! obfuscations and hacks that are designed in such a manner that they can be read by WC3's
//! built-in MPQ implementation, but will trip up other implementations.
//! 
//! **No effort is made to work around those "protections" in `ceres-mpq`**. In particular,
//! `ceres-mpq` is likely to fail when trying to read a protected MPQ which has explicitly
//! subverted the MPQ archive structure in some manner.
//! 
//! If you need a library with good support for reading protected maps, please refer to [StormLib](http://www.zezula.net/en/mpq/stormlib.html).
//!
//! # Example
//!
//! ```
//! # use ceres_mpq::Creator;
//! # use ceres_mpq::FileOptions;
//! # use ceres_mpq::Archive;
//! # use std::io::{Cursor, Read, Write, Seek, SeekFrom};
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<Error>> {
//! let buf: Vec<u8> = Vec::new();
//! let mut cursor = Cursor::new(buf);
//! 
//! // creating an archive
//! let mut creator = Creator::default();
//! creator.add_file("hello.txt", "hello world!", 
//!     FileOptions {
//!         encrypt: false, 
//!         compress: true, 
//!         adjust_key: false
//!     }
//! );
//! creator.write(&mut cursor)?;
//! 
//! cursor.seek(SeekFrom::Start(0))?;
//! 
//! // reading an archive
//! let mut archive = Archive::open(&mut cursor)?;
//! let file = archive.read_file("hello.txt")?;
//! 
//! assert_eq!(file.as_slice(), b"hello world!");
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]

pub(crate) mod consts;
pub(crate) mod header;
pub(crate) mod seeker;
pub(crate) mod table;
pub(crate) mod util;

pub mod archive;
pub mod creator;
pub mod error;

pub use archive::Archive;
pub use creator::Creator;
pub use creator::FileOptions;
pub use error::Error;