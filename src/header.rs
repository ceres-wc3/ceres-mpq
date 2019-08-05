use std::io::{Read, Seek};

use byteorder::{ReadBytesExt, LE};

use super::error::MpqError;

#[derive(Debug)]
pub(crate) struct MpqFileHeader {
    pub(crate) header_size: u32,
    pub(crate) archive_size: u32,
    pub(crate) format_version: u16,
    pub(crate) block_size: u16,
    pub(crate) hash_table_offset: u32,
    pub(crate) block_table_offset: u32,
    pub(crate) hash_table_entries: u32,
    pub(crate) block_table_entries: u32,
}

impl MpqFileHeader {
    pub(crate) fn new<R: Read + Seek>(mut reader: R) -> Result<MpqFileHeader, MpqError> {
        let header_size = reader.read_u32::<LE>()?;
        let archive_size = reader.read_u32::<LE>()?;
        let format_version = reader.read_u16::<LE>()?;
        let block_size = reader.read_u16::<LE>()?;
        let hash_table_offset = reader.read_u32::<LE>()?;
        let block_table_offset = reader.read_u32::<LE>()?;
        let hash_table_entries = reader.read_u32::<LE>()?;
        let block_table_entries = reader.read_u32::<LE>()?;

        if format_version != 0 {
            return Err(MpqError::UnsupportedVersion);
        }

        Ok(MpqFileHeader {
            header_size,
            archive_size,
            format_version,
            block_size,
            hash_table_offset,
            block_table_offset,
            hash_table_entries,
            block_table_entries,
        })
    }
}

#[derive(Debug)]
pub struct MpqUserHeader {
    pub(crate) user_data_size: u32,
    pub(crate) file_header_offset: u32,
}

impl MpqUserHeader {
    pub fn new<R: Read>(mut reader: R) -> Result<MpqUserHeader, MpqError> {
        let user_data_size = reader.read_u32::<LE>()?;
        let file_header_offset = reader.read_u32::<LE>()?;

        Ok(MpqUserHeader {
            user_data_size,
            file_header_offset,
        })
    }
}
