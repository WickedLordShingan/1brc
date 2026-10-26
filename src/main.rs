#![allow(unused)]

//TODO: remove name check (bytewise compare)
use memmap2::Mmap;
use std::cmp::min_by_key;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::thread;

const TABLE_SIZE: usize = 1 << 11;
const TABLE_MASK: usize = TABLE_SIZE - 1;

#[derive(Default, Clone, Copy)]
struct Stats<'a> {
    first_8: u64,
    len: u8,
    name: &'a [u8],
    min: i16,
    max: i16,
    count: u32,
    sum: i64,
}

// #[inline(always)]
fn str_to_u64_le(name: &'_ [u8]) -> u64 {
    let mut buf = [0u8; 8];
    let bytes = name;
    let len = bytes.len().min(8);
    buf[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(buf)
}

impl<'a> Stats<'a> {
    fn from(name: &'a [u8]) -> Self {
        Stats {
            name,
            first_8: str_to_u64_le(name),
            len: name.len() as u8,
            min: i16::MAX,
            max: i16::MIN,
            count: 0,
            sum: 0,
        }
    }
}

// #[inline(never)]
fn hash(name: &[u8]) -> u16 {
    let mut h: u32 = 0x811c9dc5;
    for &b in name {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    (h ^ (h >> 16)) as u16
}

// #[inline(never)]
// fn check(e: &Stats<'_>, name: &'_ [u8]) -> (bool, bool) {
//     (
//         e.name.is_empty(),
//         e.first_8 == str_to_u64_le(name) && e.name == name,
//     )
// }

fn lookup_or_insert<'a>(
    table: &mut [Stats<'a>],
    name: &'a [u8],
    min: i16,
    max: i16,
    sum: i64,
    count: u32,
) {
    let hash = hash(name);
    let mut idx = (hash as usize) & TABLE_MASK;

    loop {
        let (is_empty, is_match) = {
            let e = &table[idx];
            (
                e.name.is_empty(),
                e.len as usize == name.len()
                    && e.first_8 == str_to_u64_le(name)
                    && (e.len <= 8 || e.name == name),
            )
        };

        if is_empty {
            //HERE - insertion
            table[idx] = Stats::from(name);
            break;
        }
        if is_match {
            break;
        }
        idx = (idx + 1) & TABLE_MASK;
    }

    let entry = &mut table[idx];
    entry.min = entry.min.min(min);
    entry.max = entry.max.max(max);
    entry.sum += sum;
    entry.count += count;
}

// #[inline(never)]
fn insert_reading<'a>(table: &mut [Stats<'a>], name: &'a [u8], value: i16) {
    //HERE:
    lookup_or_insert(table, name, value, value, value as i64, 1);
}

fn main() {
    let file_name = "./create_file/measurements.txt";
    let file = File::open(file_name).unwrap();
    let mapped_file = unsafe { Mmap::map(&file).unwrap() };
    let size = fs::metadata(file_name).unwrap().len();
    let logical_cores = std::thread::available_parallelism().unwrap().get();

    let approx_chunk = size / logical_cores as u64;

    let mut boundaries: Vec<u64> = vec![0; logical_cores + 1];
    boundaries[logical_cores] = size;
    for i in 1..logical_cores {
        boundaries[i] = (i as u64) * approx_chunk;
    }

    thread::scope(|s| {
        for (ind, elem) in boundaries.iter_mut().enumerate() {
            if ind == 0 || ind == logical_cores {
                continue;
            }
            let data = &mapped_file[..];
            let mut current = *elem;
            s.spawn(move || {
                while data[current as usize] != b'\n' {
                    current += 1;
                }
                *elem = current + 1;
            });
        }
    });

    let mut maps: Vec<Vec<Stats>> = Vec::new();
    for _ in 0..logical_cores {
        maps.push(vec![Stats::default(); TABLE_SIZE]);
    }

    thread::scope(|s| {
        for (ind, map) in maps.iter_mut().enumerate() {
            let start = boundaries[ind] as usize;
            let end = boundaries[ind + 1] as usize;

            let data = &mapped_file[start..end];
            s.spawn(move || {
                let mut ind = 0;
                let length = data.len();
                let mut last_new_line = 0;
                let mut ind = 0;

                while last_new_line < data.len() {
                    // find ';' starting from ind
                    let semi = if ind + 8 <= data.len() {
                        let svar_reg = u64::from_le_bytes(data[ind..ind + 8].try_into().unwrap());
                        let t = svar_reg ^ 0x3B3B_3B3B_3B3B_3B3B; // 0x3B = ';'
                        let m = t.wrapping_sub(0x0101_0101_0101_0101) & !t & 0x8080_8080_8080_8080;
                        if m == 0 {
                            ind += 8;
                            continue;
                        }
                        ind + (m.trailing_zeros() / 8) as usize //trailing zeros because little
                    //endian
                    } else {
                        let mut s = ind;
                        while data[s] != b';' {
                            s += 1;
                        }
                        s
                    };

                    let station_name = &data[last_new_line..semi];

                    let mut pos = semi + 1;
                    let mut neg: i16 = 1;
                    if data[pos] == b'-' {
                        neg = -1;
                        pos += 1;
                    }

                    let mut num: i16 = 0;
                    while data[pos] != b'\n' {
                        if data[pos] != b'.' {
                            num = num * 10 + (data[pos] - b'0') as i16;
                        }
                        pos += 1;
                    }

                    //HERE - I am just inserting
                    insert_reading(map, station_name, num * neg);

                    last_new_line = pos + 1;
                    ind = last_new_line;
                }
            });
        }
    });

    let mut final_map: Vec<Stats> = vec![Stats::default(); TABLE_SIZE];

    for map in &maps {
        for entry in map.iter() {
            if !entry.name.is_empty() {
                lookup_or_insert(
                    &mut final_map,
                    entry.name,
                    entry.min,
                    entry.max,
                    entry.sum,
                    entry.count,
                );
            }
        }
    }

    let sorted: BTreeMap<&[u8], Stats> = final_map
        .into_iter()
        .filter(|s| !s.name.is_empty())
        .map(|s| (s.name, s))
        .collect();

    for (name, stats) in &sorted {
        println!(
            "{}={:.1}/{:.1}/{:.1}",
            std::str::from_utf8(name).unwrap(),
            stats.min as f64 / 10.0,
            (stats.sum as f64 / stats.count as f64) / 10.0,
            stats.max as f64 / 10.0
        );
    }
}
