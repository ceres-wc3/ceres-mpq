use std::fs;
use std::io::{Read, Seek};

use super::error::*;
use super::seeker::*;
use super::table::*;
use super::util::*;

#[derive(Debug)]
/// Implementation of a MoPaQ archive viewer.
/// 
/// Refer to top-level documentation to see which features are supported.
/// 
/// Will work on any reader that implements `Read + Seek`.
pub struct Archive<R: Read + Seek> {
    seeker: Seeker<R>,
    hash_table: FileHashTable,
    block_table: FileBlockTable,
}

impl<R: Read + Seek> Archive<R> {
    /// Try to open an MPQ archive from the specified `reader`.
    /// 
    /// Immediately, this will perform the following:
    /// 
    /// 1. Locate an MPQ header.
    /// 2. Locate and read the Hash Table.
    /// 3. Locate and read the Block Table.
    /// 
    /// If any of these steps fail, the archive is deemed corrupted and
    /// an appropriate error is returned.
    /// 
    /// No other operations will be performed.
    pub fn open(reader: R) -> Result<Archive<R>, Error> {
        let mut seeker = Seeker::new(reader)?;

        let hash_table = FileHashTable::from_seeker(&mut seeker)?;
        let block_table = FileBlockTable::from_seeker(&mut seeker)?;

        Ok(Archive {
            seeker,
            hash_table,
            block_table,
        })
    }

    /// Read a file's contents. 
    /// 
    /// Notably, the filename resolution algorithm
    /// is case-insensitive, and will treat backslashes (`\`) and forward slashes (`/`)
    /// as the same character.
    /// 
    /// Does not support single-unit files or uncompressed files.
    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, Error> {
        // find the hash entry and use it to find the block entry
        let hash_entry = self
            .hash_table
            .find_entry(name)
            .ok_or(Error::FileNotFound)?;
        let block_entry = self
            .block_table
            .get(hash_entry.block_index as usize)
            .ok_or(Error::FileNotFound)?;

        // calculate the file key
        let encryption_key = if block_entry.is_encrypted() {
            Some(calculate_file_key(
                name,
                block_entry.file_pos as u32,
                block_entry.uncompressed_size as u32,
                block_entry.is_key_adjusted(),
            ))
        } else {
            None
        };

        // read the sector offsets
        let sector_offsets = SectorOffsets::from_reader(
            &mut self.seeker,
            block_entry,
            encryption_key.map(|k| k - 1),
        )?;

        // read out all the sectors
        let sector_range = sector_offsets.all();
        let raw_data = self.seeker.read(
            block_entry.file_pos + u64::from(sector_range.0),
            u64::from(sector_range.1),
        )?;

        let mut result = Vec::with_capacity(block_entry.uncompressed_size as usize);

        let first_sector_offset = sector_offsets.one(0).unwrap().0;
        for i in 0..sector_offsets.count() {
            let sector_offset = sector_offsets.one(i).unwrap();
            let slice_start = (sector_offset.0 - first_sector_offset) as usize;
            let slice_end = slice_start + sector_offset.1 as usize;

            // if this is the last sector, then its size will be less than
            // one archive sector size, so account for that
            let uncompressed_size = if (i + 1) == sector_offsets.count() {
                let mut size = block_entry.uncompressed_size % self.seeker.info().sector_size;

                if size == 0 {
                    size = self.seeker.info().sector_size
                }
                size
            } else {
                self.seeker.info().sector_size
            };

            // decode the block and append it to the final result buffer
            let decoded_sector = decode_mpq_block(
                &raw_data[slice_start..slice_end],
                uncompressed_size,
                encryption_key.map(|k| k + i as u32),
            )?;

            result.extend(decoded_sector.iter());
        }

        Ok(result)
    }

    /// If the archive contains a `(listfile)`, this will method
    /// parse it and return a `Vec` containing all known filenames.
    pub fn files(&mut self) -> Option<Vec<String>> {
        let listfile = self.read_file("(listfile)").ok()?;

        let mut list = Vec::new();
        let mut line_start = 0;
        for i in 0..listfile.len() {
            let byte = listfile[i];

            if byte == b'\r' || byte == b'\n' {
                if i - line_start > 0 {
                    let line = &listfile[line_start..i];
                    let line = std::str::from_utf8(line);

                    if let Ok(line) = line {
                        list.push(line.to_string());
                    }
                }

                line_start = i + 1;
            }
        }

        Some(list)
    }
}

pub fn test_archive() {
    // let file = fs::File::open("yarpb1.w3x").unwrap();
    // let reader = BufReader::new(file);
    // println!("READING REFERENCE >>>>>");
    let buf = fs::read("guhun-beta8.w3x").unwrap();
    let reader = std::io::Cursor::new(buf);

    let mut archive = Archive::open(reader).unwrap();

    // hexdump::hexdump(&archive.read_file("test1.txt").unwrap());
    // hexdump::hexdump(&archive.read_file("(listfile)").unwrap());

    // println!("READING TEST >>>>>");
    // let buf = fs::read("out.w3x").unwrap();
    // let reader = std::io::Cursor::new(buf);

    // let mut archive = Archive::open(reader).unwrap();

    // hexdump::hexdump(&archive.read_file("test1.txt").unwrap());
    // hexdump::hexdump(&archive.read_file("(listfile)").unwrap());

    let files = archive.files().unwrap();

    let mut total_size = 0;
    for file_name in &files {
        let file = archive.read_file(file_name);

        if file.is_err() {
            println!(
                "file {} failed to load with err {:?}",
                file_name,
                file.err().unwrap()
            );
        } else if let Ok(file) = file {
            total_size += file.len();
        }
    }

    println!("total decompressed size: {}", total_size);
}
