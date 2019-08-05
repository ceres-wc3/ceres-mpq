use byte_slice_cast::*;
use lazy_static::lazy_static;

use super::consts::*;
use super::error::*;

lazy_static! {
    static ref CRYPTO_TABLE: [u32; 0x500] = generate_crypto_table();
}

fn generate_crypto_table() -> [u32; 0x500] {
    let mut crypto_table = [0u32; 0x500];
    let mut seed: u32 = 0x0010_0001;

    for i in 0..0x100 {
        for j in 0..5 {
            let index = i + j * 0x100;
            seed = (seed * 125 + 3) % 0x002A_AAAB;
            let t1 = (seed & 0xFFFF) << 0x10;
            seed = (seed * 125 + 3) % 0x002A_AAAB;
            let t2 = seed & 0xFFFF;

            crypto_table[index] = t1 | t2;
        }
    }

    crypto_table
}

fn hash_string_with_table(source: &[u8], hash_type: u32, lookup: &[u8]) -> u32 {
    let mut seed1: u32 = 0x7FED_7FED;
    let mut seed2: u32 = 0xEEEE_EEEE;

    for byte in source {
        let upper = u32::from(lookup[*byte as usize]);

        seed1 = CRYPTO_TABLE[(hash_type + upper) as usize] ^ (seed1.overflowing_add(seed2)).0;
        seed2 = upper
            .overflowing_add(seed1)
            .0
            .overflowing_add(seed2)
            .0
            .overflowing_add(seed2 << 5)
            .0
            .overflowing_add(3)
            .0;
    }

    seed1
}

pub(crate) fn hash_string(source: &[u8], hash_type: u32) -> u32 {
    hash_string_with_table(source, hash_type, &ASCII_UPPER_LOOKUP)
}

pub(crate) fn hash_string_noslash(source: &[u8], hash_type: u32) -> u32 {
    hash_string_with_table(source, hash_type, &ASCII_UPPER_LOOKUP_NOSLASH)
}

pub(crate) fn decrypt_mpq_block(data: &mut [u8], mut key: u32) {
    let iterations = data.len() >> 2;

    let mut key_secondary: u32 = 0xEEEE_EEEE;
    let mut temp: u32;

    // if the buffer is not aligned to u32s we need to truncate it
    // this is ok because the last bytes that don't fit into the
    // aligned slice are not encrypted
    let u32_data = &mut data[..iterations * 4].as_mut_slice_of::<u32>().unwrap();

    for i in 0..iterations {
        key_secondary = key_secondary
            .overflowing_add(CRYPTO_TABLE[(MPQ_HASH_KEY2_MIX + (key & 0xFF)) as usize])
            .0;

        u32_data[i] ^= key.overflowing_add(key_secondary).0;
        temp = u32_data[i];

        key = ((!key << 0x15).overflowing_add(0x1111_1111).0) | (key >> 0x0B);
        key_secondary = temp
            .overflowing_add(key_secondary)
            .0
            .overflowing_add(key_secondary << 5)
            .0
            .overflowing_add(3)
            .0;
    }
}

pub(crate) fn get_plain_name(input: &str) -> &[u8] {
    let bytes = input.as_bytes();
    let mut out = input.as_bytes();

    for i in 0..bytes.len() {
        if bytes[i] == b'\\' || bytes[i] == b'/' {
            out = &bytes[(i + 1)..];
        }
    }

    out
}

pub(crate) fn calculate_file_key(
    file_name: &str,
    file_offset: u32,
    file_size: u32,
    adjusted: bool,
) -> u32 {
    let plain_name = get_plain_name(file_name);
    let mut key = hash_string(plain_name, MPQ_HASH_FILE_KEY);

    if adjusted {
        key = (key + file_offset) ^ file_size
    }

    key
}

/// This will try to perform the following two operations:
/// 1) If `encryption_key` is specified, it will decrypt the block using
/// that encryption key.
/// 2) If `compressed_size` != `uncompressed_size`, it will try to decompress
/// the block. MPQ supports multiple compression types, and the compression
/// type used for a particular block is specified in the first byte of the block
/// as a set of bitflags.
pub(crate) fn decode_mpq_block(
    input: &[u8],
    uncompressed_size: u64,
    encryption_key: Option<u32>,
) -> Result<Vec<u8>, MpqError> {
    let compressed_size = input.len() as u64;
    let mut buf: Vec<u8> = input.into();

    if let Some(encryption_key) = encryption_key {
        decrypt_mpq_block(&mut buf, encryption_key);
    }

    if compressed_size != uncompressed_size {
        let compression_type = buf[0];

        if compression_type & COMPRESSION_IMA_ADCPM_MONO != 0 {
            return Err(MpqError::UnsupportedCompression {
                kind: "IMA ADCPM Mono".to_string(),
            });
        }

        if compression_type & COMPRESSION_IMA_ADCPM_STEREO != 0 {
            return Err(MpqError::UnsupportedCompression {
                kind: "IMA ADCPM Stereo".to_string(),
            });
        }

        if compression_type & COMPRESSION_HUFFMAN != 0 {
            return Err(MpqError::UnsupportedCompression {
                kind: "Huffman".to_string(),
            });
        }

        if compression_type & COMPRESSION_PKWARE != 0 {
            return Err(MpqError::UnsupportedCompression {
                kind: "PKWare DCL".to_string(),
            });
        }

        if compression_type & COMPRESSION_BZIP2 != 0 {
            let mut decompressed = vec![0u8; uncompressed_size as usize];
            let mut decompressor = bzip2::Decompress::new(false);
            let status = decompressor.decompress(&buf[1..], &mut decompressed);

            if !(status.is_ok() && status.unwrap() == bzip2::Status::Ok) {
                return Err(MpqError::Corrupted);
            }

            decompressed.resize(decompressor.total_out() as usize, 0);
            buf = decompressed;
        }

        if compression_type & COMPRESSION_ZLIB != 0 {
            let mut decompressed = vec![0u8; uncompressed_size as usize];
            let mut decompressor = flate2::Decompress::new(true);
            let status = decompressor.decompress(
                &buf[1..],
                &mut decompressed,
                flate2::FlushDecompress::Finish,
            );

            if !(status.is_ok() && status.unwrap() != flate2::Status::BufError) {
                return Err(MpqError::Corrupted);
            }

            decompressed.resize(decompressor.total_out() as usize, 0);
            buf = decompressed;
        }
    }

    Ok(buf)
}
