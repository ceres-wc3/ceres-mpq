use std::io::{Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, LE};

use super::consts::*;
use super::error::MpqError;
use super::header::*;

#[derive(Debug)]
pub(crate) struct MpqSeeker<R: Read + Seek> {
    reader: R,
    archive_info: ArchiveInfo,
}

impl<R: Read + Seek> MpqSeeker<R> {
    pub(crate) fn new(mut reader: R) -> Result<MpqSeeker<R>, MpqError> {
        let archive_info = find_headers(&mut reader)?;

        Ok(MpqSeeker {
            reader,
            archive_info,
        })
    }

    fn archive_offset(&self, offset: u64) -> u64 {
        offset + self.archive_info.header_offset
    }

    pub(crate) fn info(&self) -> &ArchiveInfo {
        &self.archive_info
    }

    pub(crate) fn read(&mut self, offset: u64, size: u64) -> Result<Vec<u8>, MpqError> {
        let offset = self.archive_offset(offset);

        if offset + size > self.archive_info.file_size {
            return Err(MpqError::Corrupted);
        }

        self.reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; size as usize];
        self.reader.read_exact(&mut buf)?;

        Ok(buf)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct TableInfo {
    pub(crate) entries: u64,
    pub(crate) offset: u64,
    pub(crate) size: u64,
}

#[derive(Debug)]
pub(crate) struct ArchiveInfo {
    pub(crate) hash_table_info: TableInfo,
    pub(crate) block_table_info: TableInfo,

    pub(crate) sector_size: u64,
    pub(crate) file_size: u64,
    pub(crate) archive_size: u64,
    pub(crate) header_offset: u64,
}

impl ArchiveInfo {
    fn new(file_size: u64, header_offset: u64, header: &MpqFileHeader) -> ArchiveInfo {
        let hash_table_info = TableInfo {
            entries: u64::from(header.hash_table_entries),
            offset: u64::from(header.hash_table_offset),
            size: u64::from(header.block_table_offset - header.hash_table_offset),
        };

        let block_table_info = TableInfo {
            entries: u64::from(header.block_table_entries),
            offset: u64::from(header.block_table_offset),
            size: u64::from(header.archive_size - header.block_table_offset),
        };

        let archive_size = u64::from(header.archive_size);
        let sector_size = 512 * 2u64.pow(u32::from(header.block_size));

        ArchiveInfo {
            hash_table_info,
            block_table_info,
            sector_size,
            file_size,
            archive_size,
            header_offset,
        }
    }
}

fn find_headers<R: Read + Seek>(mut reader: R) -> Result<ArchiveInfo, MpqError> {
    let file_size = reader.seek(SeekFrom::End(0))?;

    let mut header: Option<MpqFileHeader> = None;
    let mut file_header_offset: u64 = 0;
    for i in 0..(file_size / HEADER_BOUNDARY) {
        reader.seek(SeekFrom::Start(i * HEADER_BOUNDARY))?;

        let magic = reader.read_u32::<LE>()?;

        if magic == HEADER_USER_MAGIC {
            let user_header = MpqUserHeader::new(&mut reader)?;
            let user_header_offset = i * HEADER_BOUNDARY;
            file_header_offset = u64::from(user_header.file_header_offset) + user_header_offset;

            if file_header_offset < file_size {
                reader.seek(SeekFrom::Start(file_header_offset))?;

                let magic = reader.read_u32::<LE>()?;

                if magic != HEADER_MPQ_MAGIC {
                    return Err(MpqError::Corrupted);
                }

                let file_header = MpqFileHeader::new(&mut reader)?;
                header = Some(file_header);
                break;
            } else {
                return Err(MpqError::Corrupted);
            }
        } else if magic == HEADER_MPQ_MAGIC {
            let file_header = MpqFileHeader::new(&mut reader)?;

            file_header_offset = i * HEADER_BOUNDARY;
            header = Some(file_header);
            break;
        }
    }

    if let Some(header) = header {
        Ok(ArchiveInfo::new(file_size, file_header_offset, &header))
    } else {
        Err(MpqError::NoHeader)
    }
}
