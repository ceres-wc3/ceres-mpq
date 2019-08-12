use std::io::Error as IoError;
use std::io::{Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use super::consts::*;
use super::error::Error;

#[derive(Debug)]
pub(crate) struct FileHeader {
    pub header_size: u32,
    pub archive_size: u32,
    pub format_version: u16,
    pub block_size: u16,
    pub hash_table_offset: u32,
    pub block_table_offset: u32,
    pub hash_table_entries: u32,
    pub block_table_entries: u32,
}

impl FileHeader {
    pub fn new_v1(
        archive_size: u32,
        block_size: u32,
        hash_table_offset: u32,
        block_table_offset: u32,
        hash_table_entries: u32,
        block_table_entries: u32,
    ) -> FileHeader {
        let mut block_size = block_size / 512;
        let mut pow = 1;
        while block_size > 1 {
            block_size /= 2;
            pow += 1;
        }

        FileHeader {
            format_version: 0,
            header_size: HEADER_MPQ_SIZE as u32,
            archive_size,
            block_size: pow,
            hash_table_offset,
            hash_table_entries,
            block_table_offset,
            block_table_entries,
        }
    }

    pub fn from_reader<R: Read>(mut reader: R) -> Result<FileHeader, Error> {
        let header_size = reader.read_u32::<LE>()?;
        let archive_size = reader.read_u32::<LE>()?;
        let format_version = reader.read_u16::<LE>()?;
        let block_size = reader.read_u16::<LE>()?;
        let hash_table_offset = reader.read_u32::<LE>()?;
        let block_table_offset = reader.read_u32::<LE>()?;
        let hash_table_entries = reader.read_u32::<LE>()?;
        let block_table_entries = reader.read_u32::<LE>()?;

        if format_version != 0 {
            return Err(Error::UnsupportedVersion);
        }

        Ok(FileHeader {
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

    pub fn write<W: Write>(&self, mut writer: W) -> Result<(), IoError> {
        writer.write_u32::<LE>(HEADER_MPQ_MAGIC)?;
        writer.write_u32::<LE>(self.header_size)?;
        writer.write_u32::<LE>(self.archive_size)?;
        writer.write_u16::<LE>(self.format_version)?;
        writer.write_u16::<LE>(self.block_size)?;
        writer.write_u32::<LE>(self.hash_table_offset)?;
        writer.write_u32::<LE>(self.block_table_offset)?;
        writer.write_u32::<LE>(self.hash_table_entries)?;
        writer.write_u32::<LE>(self.block_table_entries)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct UserHeader {
    pub(crate) user_data_size: u32,
    pub(crate) file_header_offset: u32,
}

impl UserHeader {
    pub fn new<R: Read>(mut reader: R) -> Result<UserHeader, Error> {
        let user_data_size = reader.read_u32::<LE>()?;
        let file_header_offset = reader.read_u32::<LE>()?;

        Ok(UserHeader {
            user_data_size,
            file_header_offset,
        })
    }
}
