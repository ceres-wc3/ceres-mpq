pub fn sector_count_from_size(size: u64, sector_count: u64) -> u64 {
    ((size - 1) / sector_count) + 1
}
