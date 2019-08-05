use std::io::{Read, Seek};

use byteorder::{ReadBytesExt, LE};

use super::consts::*;
use super::crypto::*;
use super::error::MpqError;
use super::seeker::*;

#[derive(Debug)]
pub(crate) struct MpqHashTable {
    entries: Vec<HashTableEntry>,
}

impl MpqHashTable {
    pub(crate) fn new<R>(seeker: &mut MpqSeeker<R>) -> Result<MpqHashTable, MpqError>
    where
        R: Read + Seek,
    {
        let info = seeker.info().hash_table_info;
        let expected_size = info.entries * u64::from(HASH_TABLE_ENTRY_SIZE);
        let raw_data = seeker.read(info.offset, info.size)?;
        let decoded_data = decode_mpq_block(&raw_data, expected_size, Some(HASH_TABLE_KEY))?;

        let mut entries = Vec::with_capacity(info.entries as usize);
        let mut slice = &decoded_data[..];
        for _ in 0..info.entries {
            entries.push(HashTableEntry::from_reader(&mut slice)?);
        }

        Ok(MpqHashTable { entries })
    }

    pub(crate) fn find_entry(&self, name: &str) -> Option<&HashTableEntry> {
        let hash_mask = self.entries.len() - 1;
        let part_a = hash_string_noslash(name.as_bytes(), MPQ_HASH_NAME_A);
        let part_b = hash_string_noslash(name.as_bytes(), MPQ_HASH_NAME_B);
        let index = hash_string_noslash(name.as_bytes(), MPQ_HASH_TABLE_INDEX) as usize;

        let start_index = index & hash_mask;
        let mut index = start_index;

        loop {
            let inspected = &self.entries[index];

            if inspected.block_index == HASH_TABLE_EMPTY_ENTRY {
                break;
            }

            if inspected.hash_a == part_a && inspected.hash_b == part_b && inspected.locale == 0 {
                return Some(inspected);
            }

            index = (index + 1) & hash_mask;
            if index == start_index {
                break;
            }
        }

        None
    }
}

#[derive(Debug)]
pub(crate) struct HashTableEntry {
    pub(crate) hash_a: u32,
    pub(crate) hash_b: u32,
    pub(crate) locale: u16,
    pub(crate) platform: u16,
    pub(crate) block_index: u32,
}

impl HashTableEntry {
    fn from_reader<R: Read>(mut reader: R) -> Result<HashTableEntry, MpqError> {
        let hash_a = reader.read_u32::<LE>()?;
        let hash_b = reader.read_u32::<LE>()?;
        let locale = reader.read_u16::<LE>()?;
        let platform = reader.read_u16::<LE>()?;
        let block_index = reader.read_u32::<LE>()?;

        Ok(HashTableEntry {
            hash_a,
            hash_b,
            locale,
            platform,
            block_index,
        })
    }
}

#[derive(Debug)]
pub(crate) struct MpqBlockTable {
    entries: Vec<BlockTableEntry>,
}

impl MpqBlockTable {
    pub(crate) fn new<R>(seeker: &mut MpqSeeker<R>) -> Result<MpqBlockTable, MpqError>
    where
        R: Read + Seek,
    {
        let info = seeker.info().block_table_info;
        let expected_size = info.entries * u64::from(BLOCK_TABLE_ENTRY_SIZE);
        let raw_data = seeker.read(info.offset, info.size)?;
        let decoded_data = decode_mpq_block(&raw_data, expected_size, Some(BLOCK_TABLE_KEY))?;

        let mut entries = Vec::with_capacity(info.entries as usize);
        let mut slice = &decoded_data[..];
        for _ in 0..info.entries {
            entries.push(BlockTableEntry::from_reader(&mut slice)?);
        }

        Ok(MpqBlockTable { entries })
    }

    pub(crate) fn get(&self, index: usize) -> Option<&BlockTableEntry> {
        self.entries.get(index)
    }
}

#[derive(Debug)]
pub(crate) struct BlockTableEntry {
    pub(crate) file_pos: u64,
    pub(crate) compressed_size: u64,
    pub(crate) uncompressed_size: u64,
    pub(crate) flags: u32,
}

impl BlockTableEntry {
    fn from_reader<R: Read>(mut reader: R) -> Result<BlockTableEntry, MpqError> {
        let file_pos = u64::from(reader.read_u32::<LE>()?);
        let compressed_size = u64::from(reader.read_u32::<LE>()?);
        let uncompressed_size = u64::from(reader.read_u32::<LE>()?);
        let flags = reader.read_u32::<LE>()?;

        Ok(BlockTableEntry {
            file_pos,
            compressed_size,
            uncompressed_size,
            flags,
        })
    }

    pub(crate) fn is_imploded(&self) -> bool {
        (self.flags & MPQ_FILE_IMPLODE) != 0
    }

    pub(crate) fn is_compressed(&self) -> bool {
        (self.flags & MPQ_FILE_COMPRESS) != 0
    }

    pub(crate) fn is_encrypted(&self) -> bool {
        (self.flags & MPQ_FILE_ENCRYPTED) != 0
    }

    pub(crate) fn is_key_adjusted(&self) -> bool {
        (self.flags & MPQ_FILE_ADJUST_KEY) != 0
    }
}

#[derive(Debug)]
pub(crate) struct SectorOffsets {
    offsets: Vec<u32>,
}

impl SectorOffsets {
    pub(crate) fn new<R>(
        seeker: &mut MpqSeeker<R>,
        block_entry: &BlockTableEntry,
        encryption_key: Option<u32>,
    ) -> Result<SectorOffsets, MpqError>
    where
        R: Read + Seek,
    {
        let sector_count = ((block_entry.uncompressed_size - 1) / seeker.info().sector_size) + 1;
        let mut raw_data = seeker.read(block_entry.file_pos, (sector_count + 1) * 4)?;

        if let Some(encryption_key) = encryption_key {
            decrypt_mpq_block(&mut raw_data, encryption_key);
        }

        let mut slice = &raw_data[..];
        let mut offsets = vec![0u32; (sector_count + 1) as usize];
        for i in 0..=sector_count {
            offsets[i as usize] = slice.read_u32::<LE>()?;
        }

        Ok(SectorOffsets { offsets })
    }

    pub(crate) fn one(&self, index: usize) -> Option<(u32, u32)> {
        if index >= (self.offsets.len() - 1) {
            None
        } else {
            Some((
                self.offsets[index],
                self.offsets[index + 1] - self.offsets[index],
            ))
        }
    }

    pub(crate) fn all(&self) -> (u32, u32) {
        let len = self.offsets.len();

        (self.offsets[0], self.offsets[len - 1] - self.offsets[0])
    }

    pub(crate) fn count(&self) -> usize {
        self.offsets.len() - 1
    }
}
