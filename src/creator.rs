use std::borrow::Cow;
use std::cmp::min;
use std::io::Error as IoError;
use std::io::{Seek, SeekFrom, Write};

use byteorder::{WriteBytesExt, LE};
use indexmap::IndexMap;

// use super::archive::Archive;
use super::consts::*;
use super::header::*;
use super::table::*;
use super::util::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct FileKey {
    hash_a: u32,
    hash_b: u32,
    index: u32,
}

impl FileKey {
    fn new(name: &str) -> FileKey {
        let hash_a = hash_string(name.as_bytes(), MPQ_HASH_NAME_A);
        let hash_b = hash_string(name.as_bytes(), MPQ_HASH_NAME_B);
        let index = hash_string(name.as_bytes(), MPQ_HASH_TABLE_INDEX);

        FileKey {
            hash_a,
            hash_b,
            index,
        }
    }
}

#[derive(Debug)]
struct FileRecord {
    file_name: String,
    contents: Vec<u8>,
    offset: u64,
    compressed_size: u64,
    options: FileOptions,
}

impl FileRecord {
    fn new<S: Into<String>, C: Into<Vec<u8>>>(
        name: S,
        contents: C,
        options: FileOptions,
    ) -> FileRecord {
        FileRecord {
            file_name: name.into(),
            contents: contents.into(),
            offset: 0,
            compressed_size: 0,
            options,
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Represents various options that can be used when adding a file to an archive.
pub struct FileOptions {
    /// Whether to encrypt the file using MPQ's encryption scheme.
    /// The encryption key is derived from the file name, so in practice
    /// this is pretty useless.
    pub encrypt: bool,
    /// Whether to compress the file. Currently will only try to use DEFLATE
    /// compression.
    pub compress: bool,
    /// If the file is ecnrypted, this will "adjust" the encryption key by
    /// performing some simple transformations on it. By default, this is used for
    /// "technical" files such as `(listfile)`.
    pub adjust_key: bool,
}

impl Default for FileOptions {
    fn default() -> FileOptions {
        FileOptions {
            encrypt: false,
            compress: false,
            adjust_key: false,
        }
    }
}

impl FileOptions {
    fn flags(self) -> u32 {
        let mut flags = MPQ_FILE_EXISTS;

        if self.encrypt {
            flags |= MPQ_FILE_ENCRYPTED;
        }

        if self.adjust_key {
            flags |= MPQ_FILE_ADJUST_KEY;
        }

        if self.compress {
            flags |= MPQ_FILE_COMPRESS;
        }

        flags
    }
}

#[derive(Debug)]
/// Creator capable of creating MPQ Version 1 archives.
///
/// Will hold all the files in memory until asked to [write](struct.Creator.html#method.write) them
/// to a `writer`.
///
/// When writing, a `(listfile)` will be automatically appended to the archive.
// TODO: Add support for multiple compression types
pub struct Creator {
    added_files: IndexMap<FileKey, FileRecord>,

    sector_size: u64,
}

impl Default for Creator {
    fn default() -> Creator {
        Creator {
            added_files: IndexMap::new(),
            sector_size: 0x10000,
        }
    }
}

impl Creator {
    /// Adds a file to be later written to the archive.
    ///
    /// All forward slashes (`/`) in the file path will be auto-converted to backward slashes (`\`)
    ///
    /// [`FileOptions`](struct.FileOptions.html) determine the options for adding the file, e.g. encryption and compression.
    pub fn add_file<C>(&mut self, file_name: &str, contents: C, options: FileOptions)
    where
        C: Into<Vec<u8>>,
    {
        let file_name = file_name.replace('/', "\\");
        let key = FileKey::new(&file_name);

        self.added_files
            .insert(key, FileRecord::new(file_name, contents, options));
    }

    /// Writes out the entire archive to the specified writer.
    ///
    /// The archive start position is calculated as follows:  
    /// `((current_pos + (HEADER_BOUNDARY - 1)) / HEADER_BOUNDARY) * HEADER_BOUNDARY`  
    /// Where `current_pos` is the `writer`'s current seek pos, and `HEADER_BOUNDARY` is 512.
    ///
    /// Will write the following:
    /// - MPQ Header
    /// - All files with their sector offset table
    /// - MPQ hash table
    /// - MPQ block table
    pub fn write<W>(&mut self, mut writer: W) -> Result<(), IoError>
    where
        W: Write + Seek,
    {
        let (added_files, sector_size) = match self {
            Creator {
                added_files,
                sector_size,
            } => (added_files, *sector_size),
        };

        let current_pos = writer.seek(SeekFrom::Current(0))?;
        // starting from the current pos, this will find the closest valid header position
        let archive_start =
            ((current_pos + (HEADER_BOUNDARY - 1)) / HEADER_BOUNDARY) * HEADER_BOUNDARY;
        writer.seek(SeekFrom::Start(archive_start))?;

        // skip writing the header for now
        writer.seek(SeekFrom::Current(HEADER_MPQ_SIZE as i64))?;

        // create a listfile
        let mut listfile = String::new();
        for file in added_files.values() {
            listfile += &file.file_name;
            listfile += "\r\n";
        }

        // add it to the file list
        {
            let key = FileKey::new("(listfile)");
            added_files.insert(
                key,
                FileRecord::new(
                    "(listfile)",
                    listfile,
                    FileOptions {
                        compress: true,
                        encrypt: true,
                        adjust_key: true,
                    },
                ),
            );
        }

        // write out all the files back-to-back
        for file in added_files.values_mut() {
            write_file(sector_size, archive_start, &mut writer, file)?;
        }

        let mut hashtable_size = MIN_HASH_TABLE_SIZE;
        while hashtable_size < added_files.len() {
            hashtable_size *= 2;
        }

        // write hash table and remember its position
        let hashtable_pos = write_hashtable(&mut writer, hashtable_size, &added_files)?;

        // write block table and remember its position
        let blocktable_pos = write_blocktable(&mut writer, &added_files)?;

        // write header
        let archive_end = writer.seek(SeekFrom::Current(0))?;
        write_header(
            &mut writer,
            (archive_start, archive_end),
            (hashtable_pos, hashtable_size),
            (blocktable_pos, added_files.len()),
            sector_size,
        )?;

        Ok(())
    }
}

fn write_hashtable<W>(
    mut writer: W,
    hashtable_size: usize,
    added_files: &IndexMap<FileKey, FileRecord>,
) -> Result<u64, IoError>
where
    W: Write + Seek,
{
    let hashtable_pos = writer.seek(SeekFrom::Current(0))?;
    let mut hashtable = vec![HashEntry::blank(); hashtable_size];
    let hash_index_mask = hashtable_size - 1;

    for (block_index, (key, _)) in added_files.iter().enumerate() {
        let mut hash_index = (key.index as usize) & hash_index_mask;
        let hash_entry = HashEntry::new(key.hash_a, key.hash_b, block_index as u32);

        while !hashtable[hash_index].is_blank() {
            hash_index += 1;
            if hash_index == hashtable_size {
                hash_index = 0;
            }
        }

        hashtable[hash_index] = hash_entry;
    }

    let mut buf = vec![0u8; hashtable_size * HASH_TABLE_ENTRY_SIZE as usize];

    let mut cursor = buf.as_mut_slice();
    for entry in hashtable {
        entry.write(&mut cursor)?;
    }
    encrypt_mpq_block(&mut buf, HASH_TABLE_KEY);

    writer.write_all(&buf)?;

    Ok(hashtable_pos)
}

fn write_blocktable<W>(
    mut writer: W,
    added_files: &IndexMap<FileKey, FileRecord>,
) -> Result<u64, IoError>
where
    W: Write + Seek,
{
    let blocktable_pos = writer.seek(SeekFrom::Current(0))?;

    let mut buf = vec![0u8; added_files.len() * BLOCK_TABLE_ENTRY_SIZE as usize];

    let mut cursor = buf.as_mut_slice();
    for file in added_files.values() {
        let flags = file.options.flags();

        let block_entry = BlockEntry::new(
            file.offset,
            file.compressed_size,
            file.contents.len() as u64,
            flags,
        );

        block_entry.write(&mut cursor)?;
    }

    encrypt_mpq_block(&mut buf, BLOCK_TABLE_KEY);
    writer.write_all(&buf)?;

    Ok(blocktable_pos)
}

fn write_header<W>(
    mut writer: W,
    (archive_start, archive_end): (u64, u64),
    (hashtable_pos, hashtable_size): (u64, usize),
    (blocktable_pos, blocktable_size): (u64, usize),
    sector_size: u64,
) -> Result<(), IoError>
where
    W: Write + Seek,
{
    let header = FileHeader::new_v1(
        (archive_end - archive_start) as u32,
        sector_size as u32,
        (hashtable_pos - archive_start) as u32,
        (blocktable_pos - archive_start) as u32,
        hashtable_size as u32,
        blocktable_size as u32,
    );

    writer.seek(SeekFrom::Start(archive_start))?;
    header.write(&mut writer)?;

    Ok(())
}

/// Writes out the specified file starting at the writer's current position.
/// If the file is marked for compression, a Sector Offset Table (SOT) will be written, and all sectors will attempt compression.
/// If the file is not marked for compression, no SOT will be written.
/// If the file is marked for encryption, it will also be encrypted after compression.
fn write_file<W>(
    sector_size: u64,
    archive_start: u64,
    mut writer: W,
    file: &mut FileRecord,
) -> Result<(), IoError>
where
    W: Write + Seek,
{
    let options = file.options;
    let sector_count = sector_count_from_size(file.contents.len() as u64, sector_size);
    let file_start = writer.seek(SeekFrom::Current(0))?;

    // calculate the encryption key if encryption was requested
    let encryption_key = if options.encrypt {
        Some(calculate_file_key(
            &file.file_name,
            (file_start - archive_start) as u32,
            file.contents.len() as u32,
            options.adjust_key,
        ))
    } else {
        None
    };

    if options.compress {
        let mut offsets: Vec<u32> = Vec::new();

        // store the start of the first sector and prepare to write there
        let first_sector_start = ((sector_count + 1) * 4) as u32;
        writer.seek(SeekFrom::Current(i64::from(first_sector_start)))?;
        offsets.push(first_sector_start);
        // write each sector and the offset of its end
        for i in 0..sector_count {
            let sector_start = i * sector_size;
            let sector_end = min((i + 1) * sector_size, file.contents.len() as u64);
            let data = &file.contents[sector_start as usize..sector_end as usize];

            let mut compressed = compress_mpq_block(data);

            // encrypt the block if encryption was requested
            if let Some(key) = encryption_key.map(|k| k + i as u32) {
                encrypt_mpq_block(compressed.to_mut(), key);
            }

            writer.write_all(&compressed)?;

            // store the end of the current sector
            // which is also the start of the next sector if there is one

            let current_offset = writer.seek(SeekFrom::Current(0))?;
            offsets.push((current_offset - file_start) as u32);
        }

        let file_end = writer.seek(SeekFrom::Current(0))?;

        // write the sector offset table
        {
            let mut buf = vec![0u8; offsets.len() * 4];
            let mut cursor = buf.as_mut_slice();
            for offset in &offsets {
                cursor.write_u32::<LE>(*offset)?;
            }

            // encrypt the SOT if requested
            if let Some(key) = encryption_key.map(|k| k - 1) {
                encrypt_mpq_block(&mut buf, key);
            }

            writer.seek(SeekFrom::Start(file_start))?;
            writer.write_all(&buf)?;
        }

        // put the writer at the file end, so that we don't overwrite this file with subsequent writes
        writer.seek(SeekFrom::Start(file_end))?;

        file.offset = file_start - archive_start;
        file.compressed_size = file_end - file_start;

        Ok(())
    } else {
        // write each sector
        for i in 0..sector_count {
            let sector_start = i * sector_size;
            let sector_end = min((i + 1) * sector_size, file.contents.len() as u64);
            let data = &file.contents[sector_start as usize..sector_end as usize];
            let mut buf = Cow::Borrowed(data);

            // encrypt the block if encryption was requested
            if let Some(key) = encryption_key.map(|k| k + i as u32) {
                encrypt_mpq_block(buf.to_mut(), key);
            }

            writer.write_all(&buf)?;
        }

        let file_end = writer.seek(SeekFrom::Current(0))?;

        file.offset = file_start - archive_start;
        file.compressed_size = file_end - file_start;

        Ok(())
    }
}
