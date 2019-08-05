use std::collections::HashMap;
use std::io::{Seek, Write};

use super::archive::MpqReader;
use super::consts::*;
use super::crypto::*;

#[derive(Debug, PartialEq, Eq, Hash)]
struct FileKey {
    hash_a: u32,
    hash_b: u32,
    index: u32,
}

impl FileKey {
    fn new(name: &str) -> FileKey {
        let hash_a = hash_string_noslash(name.as_bytes(), MPQ_HASH_NAME_A);
        let hash_b = hash_string_noslash(name.as_bytes(), MPQ_HASH_NAME_B);
        let index = hash_string_noslash(name.as_bytes(), MPQ_HASH_TABLE_INDEX);

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
}

impl FileRecord {
    fn new<S: Into<String>, C: Into<Vec<u8>>>(name: S, contents: C) -> FileRecord {
        FileRecord {
            file_name: name.into(),
            contents: contents.into(),
        }
    }
}

#[derive(Debug)]
pub struct MpqBuilder {
    added_files: HashMap<FileKey, FileRecord>,
}

impl MpqBuilder {
    pub fn new() -> MpqBuilder {
        unimplemented!()
    }

    pub fn add_file<C>(&mut self, file_name: &str, contents: C)
    where
        C: Into<Vec<u8>>,
    {
        let key = FileKey::new(file_name);

        self.added_files
            .entry(key)
            .or_insert_with(|| FileRecord::new(file_name, contents));
    }

    pub fn write<W>(self, mut writer: W)
    where
        W: Write + Seek,
    {
    }
}
