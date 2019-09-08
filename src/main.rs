extern crate winapi;
extern crate chrono;

use std::io::prelude::*;
use std::collections::HashSet;
use std::time::SystemTime;

use chrono::TimeZone;

const CHUNK_SIZE: usize = 8;
const READ_BUF_SIZE: usize = 4096;
const HASH_ALGORITHM: &'static str = "SHA256";
const SAVE_DIR: &'static str = r#"C:\Users\Rafael\Pictures\Spotlight"#;
const SPOTLIGHT_DIR: &'static str = r#"C:\Users\Rafael\AppData\Local\Packages\Microsoft.Windows.ContentDeliveryManager_cw5n1h2txyewy\LocalState\Assets"#;

struct SuffixGenerator {
    path_array: Vec<u8>,
    original_size: isize,
    extension: String,
    array_pos: isize,
    ascii_lower: Box<dyn Iterator<Item=u8>>
}

impl SuffixGenerator {
    pub fn new(path: &std::path::PathBuf, ext: &str) -> SuffixGenerator {
        let mut path_str: String = path.to_str().expect("Invalid unicode").to_string();
        let path_array: Vec<u8> = unsafe { path_str.as_bytes_mut().to_vec() };
        let original_size = path_array.len() as isize;
        let mut extension = String::from(".");
        extension.push_str(ext);
        let ascii_iter = Box::new((97u8..=122u8).chain(std::iter::once(0u8)).cycle());
        SuffixGenerator {
            path_array,
            original_size,
            extension,
            array_pos: -1,
            ascii_lower: ascii_iter
        }
    }

    fn adjust_suffix(&mut self) {
        // place in ::new
        if self.array_pos == -1 {
            self.path_array.push(96u8);
            self.array_pos = 0;
        }

        let mut cur_pos = self.array_pos;
        loop {
            if cur_pos < 0 {
                self.path_array.push(97u8);
                self.array_pos += 1;
                break;
            }
            let pos_offset = (cur_pos + self.original_size) as usize;
            self.path_array[pos_offset] += 1;
            if self.path_array[pos_offset] == 123u8 {
                self.path_array[pos_offset] = 97;
                cur_pos -= 1;
            } else {
                break;
            }
        }
    }
}

impl Iterator for SuffixGenerator {
    type Item = std::path::PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.ascii_lower.next() {
                Some(0) => {
                    self.adjust_suffix();
                },
                Some(ch) => {
                    self.path_array.push(ch);
                    
                    let mut tmp_str: String = String::from_utf8(self.path_array.clone()).expect("Invalid unicode");
                    tmp_str.push_str(&self.extension);
                    let result = std::path::PathBuf::from(tmp_str);

                    self.path_array.pop(); // remove ch
                    return Some(result);
                },
                None => panic!("Should be unreachable")
            }
        }
    }
}

#[inline(always)]
fn is_jpg(buf: &[u8]) -> bool {
    const JPG_SIG: [u8; 12] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
    buf == JPG_SIG
}

#[inline(always)]
fn is_landscape(buf: &[u8]) -> bool {
    buf[..2] < buf[2..]
}

#[inline]
fn hash_if_landscape_jpg(entry: &std::fs::DirEntry) -> Option<Vec<u8>> {
    let mut hasher = spotlight::WinHasher::new(HASH_ALGORITHM).unwrap();
    let mut buf: [u8; READ_BUF_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };

    let mut file = std::fs::File::open(entry.path()).unwrap();
    file.read_exact(&mut buf[..12]).expect("Read error");
    if is_jpg(&buf[..12]) {
        file.read_exact(&mut buf[12..167]).expect("Read error");
        if is_landscape(&buf[163..167]) {
            hasher.update(&mut buf[..167]).unwrap();
            while let Ok(size) = file.read(&mut buf) {
                if size == 0 {
                    break;
                }
                hasher.update(&mut buf[..size]).unwrap();
            }
            return Some(hasher.digest().unwrap());
        }
    }
    None
}

fn hash_saved_images(entries: &[std::fs::DirEntry]) -> HashSet<Vec<u8>> {
    let mut chunk_set: HashSet<Vec<u8>> = HashSet::new();
    
    for entry in entries {
        if let Some(digest) = hash_if_landscape_jpg(entry) {
            chunk_set.insert(digest);
        }
    }
    chunk_set
}

fn find_new_image(search_set: &HashSet<Vec<u8>>, entries: &[std::fs::DirEntry]) -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
    let mut result = vec![];

    for entry in entries {
        if let Some(digest) = hash_if_landscape_jpg(entry) {
            if !search_set.contains(&digest) {
                let path = entry.path();
                let creation_time = entry.metadata().unwrap().created().unwrap();
                result.push((path, creation_time));
            }
        }
    }
    result
}

fn main() {
    let save_dir = std::path::Path::new(SAVE_DIR);
    let spotlight_dir = std::path::Path::new(SPOTLIGHT_DIR);
    
    // !!find saved file first!!

    let mut search_set: HashSet<Vec<u8>> = HashSet::new();
    let mut new_images = vec![];

    let is_file = |entry: std::io::Result<std::fs::DirEntry>| -> Option<std::fs::DirEntry> {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    return Some(entry);
                }
            }
        }
        None
    };
    let save_dir_entries = save_dir.read_dir().expect("Cannot read dir")
                                   .filter_map(is_file)
                                   .collect::<Vec<_>>();

    for chunk in save_dir_entries.chunks(CHUNK_SIZE) {
        let chunk_set = hash_saved_images(chunk);
        search_set.extend(chunk_set);
    }

    let spotlight_dir_entries = spotlight_dir.read_dir().expect("Cannot read dir")
                                             .filter_map(is_file)
                                             .collect::<Vec<_>>();

    for chunk in spotlight_dir_entries.chunks(CHUNK_SIZE) {
        let mut chunk_images = find_new_image(&search_set, chunk);
        new_images.append(&mut chunk_images);
    }

    println!("New images: {}", &new_images.len());

    for (src_path, sys_time) in new_images {
        if let Ok(duration) = sys_time.duration_since(SystemTime::UNIX_EPOCH) {
            
            let nsecs = duration.as_nanos();
            let dt = chrono::Local.timestamp_nanos(nsecs as i64).naive_local();
            let date_str = dt.format("%Y%m%d").to_string();
            
            let mut path = save_dir.to_path_buf();
            path.push(date_str);

            let suffixes = SuffixGenerator::new(&path, "jpg");

            for dst_path in suffixes.take(100) {
                if !dst_path.is_file() {
                    // std::fs::copy(&src_path, &dst_path).expect("Cannot copy file");
                    dbg!(&dst_path);
                    break;
                }
            }
        }
    }

    std::process::exit(0);
}