extern crate chrono;

use std::io::prelude::*;
use std::collections::HashSet;
use std::fs::DirEntry;
use std::time::SystemTime;

use chrono::TimeZone;
use spotlight::*;

const CHUNK_SIZE: usize = 8;
const READ_BUF_SIZE: usize = 4096;
const HASH_ALGORITHM: &'static str = "SHA256";
const SAVE_DIR: &'static str = r#"C:\Users\Rafael\Pictures\Spotlight"#;
const SPOTLIGHT_DIR: &'static str = r#"C:\Users\Rafael\AppData\Local\Packages\Microsoft.Windows.ContentDeliveryManager_cw5n1h2txyewy\LocalState\Assets"#;

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
fn hash_if_landscape_jpg(entry: &DirEntry) -> Option<Vec<u8>> {
    let mut hasher = hasher::WinHasher::new(HASH_ALGORITHM).unwrap();
    // Uninitialized array. Vec<u8> works here too.
    let mut buf: [u8; READ_BUF_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };

    let mut file = std::fs::File::open(entry.path()).unwrap();
    // The JPG header is in the first 12 bytes
    file.read_exact(&mut buf[..12]).expect("Read error");
    if is_jpg(&buf[..12]) {
        // Not sure if this is true for all JPG files but
        // Windows Spotlight images store the height in file[163..165] and
        // width in file[165..167]. We're starting from [12..] here so
        // we don't have to read it again when we're already hashing the file.
        file.read_exact(&mut buf[12..167]).expect("Read error");
        // is_landscape needs only 4 bytes in the aforementioned range
        if is_landscape(&buf[163..167]) {
            // Hash the read data so far then hash the rest
            // using the while loop
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
    // not a landscape JPG
    None
}

fn hash_saved_images(entries: &[DirEntry]) -> HashSet<Vec<u8>> {
    let mut chunk_set: HashSet<Vec<u8>> = HashSet::new();
    
    for entry in entries {
        if let Some(digest) = hash_if_landscape_jpg(entry) {
            chunk_set.insert(digest);
        }
    }
    chunk_set
}

fn find_new_image(search_set: &HashSet<Vec<u8>>, entries: &[DirEntry]) -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
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

    // exclude subdirectories
    let is_file_filter = |entry: std::io::Result<DirEntry>| -> Option<DirEntry> {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    return Some(entry);
                }
            }
        }
        None
    };

    // make a vector for chunking
    let to_file_vec = |path: &std::path::Path| -> Vec<_> {
        path.read_dir().expect("Cannot read dir")
        .filter_map(is_file_filter)
        .collect::<Vec<_>>()
    };

    let save_dir_entries = to_file_vec(save_dir);
    for chunk in save_dir_entries.chunks(CHUNK_SIZE) {
        let chunk_set = hash_saved_images(chunk);
        search_set.extend(chunk_set);
    }

    let spotlight_dir_entries = to_file_vec(spotlight_dir);
    for chunk in spotlight_dir_entries.chunks(CHUNK_SIZE) {
        let mut chunk_images = find_new_image(&search_set, chunk);
        new_images.append(&mut chunk_images);
    }

    // println!("New images: {}", &new_images.len());
    let mut new_wallpaper: Option<std::path::PathBuf> = None;

    for (src_path, sys_time) in new_images {
        if let Ok(duration) = sys_time.duration_since(SystemTime::UNIX_EPOCH) {
            
            let nsecs = duration.as_nanos();
            let datetime = chrono::Local.timestamp_nanos(nsecs as i64);
            let date_str = datetime.format("%Y%m%d").to_string();
            
            let mut path = save_dir.to_path_buf();
            path.push(date_str);

            let suffixes = suffix_gen::SuffixGenerator::new(path, "jpg");

            for dst_path in suffixes.take(100) {
                if !dst_path.is_file() {
                    std::fs::copy(&src_path, &dst_path).expect("Cannot copy file");
                    // dbg!(&dst_path);
                    new_wallpaper = Some(dst_path);
                    break;
                }
            }
        }
    }

    if let Some(new_wallpaper) = new_wallpaper {
        wallpaper::set_desktop_wallpaper(new_wallpaper);
    }

    std::process::exit(0);
}