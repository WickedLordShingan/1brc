#![allow(unused)]

use memmap2::Mmap;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::thread;

struct Stats {
    min: i16,
    max: i16,
    sum: i64,
    count: u32,
}

fn main() {
    let file_name = "./dont_open/measurements.txt";
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
            if (ind == 0 || ind == logical_cores) {
                continue;
            }
            let data = &mapped_file[..];
            let mut current = *elem;
            s.spawn(move || {
                while (data[current as usize] != b'\n') {
                    current += 1;
                }
                *elem = current + 1;
            });
        }
    });

    let mut maps: Vec<HashMap<&str, Stats>> = Vec::new();
    for _ in 0..logical_cores {
        maps.push(HashMap::new());
    }

    thread::scope(|s| {
        for (ind, map) in maps.iter_mut().enumerate() {
            let start = boundaries[ind] as usize;
            let end = boundaries[ind + 1] as usize;

            let data = &mapped_file[start..end];
            s.spawn(move || {
                let mut ind = 0;
                while (ind < data.len()) {
                    let line_start = ind;
                    while (data[ind] != b'\n') {
                        ind += 1
                    }
                    let line = &data[line_start..ind + 1];
                    ind += 1;

                    //parsing the line
                    let mut ind_line: usize = 0;
                    while (line[ind_line] != b';') {
                        ind_line += 1;
                    }

                    let station_name = std::str::from_utf8(&line[..ind_line]).unwrap();
                    ind_line += 1; //skips past the ;

                    let mut negative = false;
                    if line[ind_line] == b'-' {
                        ind_line += 1;
                        negative = true;
                    }

                    let mut num = 0;
                    while (line[ind_line] != b'\n') {
                        if (line[ind_line] == b'.') {
                            ind_line += 1;
                        }
                        let digit = (line[ind_line] - b'0') as i16;
                        num = num * 10 + digit;
                        ind_line += 1;
                    }

                    if negative {
                        num *= -1;
                    };

                    map.entry(station_name)
                        .and_modify(|stats| {
                            stats.min = stats.min.min(num);
                            stats.max = stats.max.max(num);
                            stats.sum += num as i64;
                            stats.count += 1;
                        })
                        .or_insert(Stats {
                            min: num,
                            max: num,
                            sum: num as i64,
                            count: 1,
                        });
                }
            });
        }
    });

    let mut final_map: HashMap<&str, Stats> = HashMap::new();

    for map in maps {
        for (key, value) in map {
            final_map
                .entry(key)
                .and_modify(|stats| {
                    stats.min = stats.min.min(value.min);
                    stats.max = stats.max.max(value.max);
                    stats.sum += value.sum;
                    stats.count += value.count;
                })
                .or_insert(value);
        }
    }

    let sorted: BTreeMap<&str, Stats> = final_map.into_iter().collect();

    for (name, stats) in &sorted {
        println!(
            "{}={:.1}/{:.1}/{:.1}",
            name,
            stats.min as f64 / 10.0,
            (stats.sum as f64 / stats.count as f64) / 10.0,
            stats.max as f64 / 10.0
        );
    }
}
