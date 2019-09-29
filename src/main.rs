extern crate chrono;

use std::collections::HashSet;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;

use chrono::TimeZone;
use spotlight::*;

const READ_BUF_SIZE: usize = 4096;
const HASH_SIZE: usize = 32; // SHA256 is 32 bytes
const HASH_ALGORITHM: &'static str = "SHA256";
const SAVE_DIR: &'static str = r#"C:\Users\Rafael\Pictures\Spotlight"#;
const SPOTLIGHT_DIR: &'static str = r#"C:\Users\Rafael\AppData\Local\Packages\Microsoft.Windows.ContentDeliveryManager_cw5n1h2txyewy\LocalState\Assets"#;
const SAVED_HASH: &'static str = r#"C:\Users\Rafael\Pictures\Spotlight\spotlight.hashes"#;

#[inline(always)]
fn is_jpg(buf: &[u8]) -> bool {
    // The JPG header is in the first 12 bytes of the file
    const JPG_SIG: [u8; 12] = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
    ];
    buf == JPG_SIG
}

#[inline(always)]
fn is_landscape(buf: &[u8]) -> bool {
    // Tests if height < width. Not sure if this is
    // true for all JPG files but Windows Spotlight images store the
    // height in file[163..165] and width in file[165..167].
    buf[..2] < buf[2..]
}

#[inline]
fn hash_if_landscape_jpg<P: AsRef<std::path::Path>>(
    path: P,
    hasher: &mut hasher::WinHasher,
    buf: &mut [u8; READ_BUF_SIZE],
) -> Option<Vec<u8>> {
    const JPG_SIG_END: usize = 12;
    const RESOLUTION_START: usize = 163;
    const RESOLUTION_END: usize = 167;

    let mut file = std::fs::File::open(path).unwrap();
    if let Ok(mut read_size) = file.read(buf) {
        if read_size < JPG_SIG_END {
            if let Ok(_) = file.read_exact(&mut buf[read_size..JPG_SIG_END]) {
                read_size = JPG_SIG_END;
            } else {
                return None;
            }
        }
        if is_jpg(&buf[..JPG_SIG_END]) {
            if read_size < RESOLUTION_END {
                if let Ok(_) = file.read_exact(&mut buf[read_size..RESOLUTION_END]) {
                    read_size = RESOLUTION_END;
                } else {
                    return None;
                }
            }
            if is_landscape(&buf[RESOLUTION_START..RESOLUTION_END]) {
                // Hash the read data so far then hash the rest
                // using a while loop
                hasher.update(&mut buf[..read_size]).unwrap();
                while let Ok(size) = file.read(buf) {
                    if size == 0 {
                        break;
                    }
                    hasher.update(&mut buf[..size]).unwrap();
                }
                return Some(hasher.digest().unwrap());
            }
        }
    }
    // not a landscape JPG
    None
}

#[test]
fn test_jpg_hashing() {
    let path = std::path::PathBuf::from(r#"C:\Users\Rafael\Pictures\Spotlight\20190713a.jpg"#);
    assert!(path.is_file());
    let mut hasher = hasher::WinHasher::new(HASH_ALGORITHM).unwrap();
    let mut buf: [u8; READ_BUF_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    let digest = hash_if_landscape_jpg(path, &mut hasher, &mut buf).unwrap();
    let hash_string: String = digest.iter().map(|&i| format!("{:02x}", i)).collect();
    assert_eq!(
        hash_string,
        "3509c4b6f09e861bcdda4c97feb2106c3d6baba7880444783623e420e06003b2"
    );
    dbg!(hash_string);
}

fn read_saved_hashes<P: AsRef<std::path::Path>>(path: P) -> Vec<Vec<u8>> {
    let mut file = std::fs::File::open(path).unwrap();
    let mut buf: [u8; READ_BUF_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    let mut result: Vec<Vec<u8>> = vec![];

    while let Ok(mut read_size) = file.read(&mut buf) {
        if read_size == 0 {
            break;
        }
        let mut start = 0;
        while read_size >= HASH_SIZE {
            result.push(buf[start..start + HASH_SIZE].to_vec());
            start += HASH_SIZE;
            read_size -= HASH_SIZE;
        }
        file.seek(std::io::SeekFrom::Current(-(read_size as i64)))
            .unwrap();
    }
    result
}

fn main() {
    let save_dir = std::path::Path::new(SAVE_DIR);
    let spotlight_dir = std::path::Path::new(SPOTLIGHT_DIR);
    let spotlight_file = std::path::Path::new(SAVED_HASH);

    let mut search_set: HashSet<Vec<u8>> = HashSet::new();
    let mut new_images = vec![];

    let mut hasher = hasher::WinHasher::new(HASH_ALGORITHM).unwrap();
    let mut buf: [u8; READ_BUF_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };

    // exclude subdirectories
    let get_entries = |path: &std::path::Path| -> _ {
        path.read_dir().unwrap().filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        return Some(entry);
                    }
                }
            }
            None
        })
    };

    if spotlight_file.is_file() {
        let hash_list = read_saved_hashes(spotlight_file);
        search_set.reserve(hash_list.len());
        for hash in hash_list {
            search_set.insert(hash);
        }
    } else {
        for entry in get_entries(save_dir) {
            if let Some(digest) = hash_if_landscape_jpg(entry.path(), &mut hasher, &mut buf) {
                search_set.insert(digest);
            }
        }
    }

    for entry in get_entries(spotlight_dir) {
        if let Some(digest) = hash_if_landscape_jpg(entry.path(), &mut hasher, &mut buf) {
            if !search_set.contains(&digest) {
                let path = entry.path();
                let creation_time = entry.metadata().unwrap().created().unwrap();
                new_images.push((path, digest, creation_time));
            }
        }
    }

    let mut new_wallpaper: Option<PathBuf> = None;

    for (src_path, digest, sys_time) in new_images {
        // continue only if digest is not in set
        if search_set.insert(digest) {
            if let Ok(duration) = sys_time.duration_since(SystemTime::UNIX_EPOCH) {
                let nsecs = duration.as_nanos();
                let datetime = chrono::Local.timestamp_nanos(nsecs as i64);
                let date_str = datetime.format("%Y%m%d").to_string();
                let mut path = save_dir.to_path_buf();
                path.push(date_str);

                let suffixes = suffix_gen::SuffixGenerator::new(path, "jpg");

                for dst_path in suffixes.take(100) {
                    if !dst_path.is_file() {
                        // dbg!(&dst_path);
                        std::fs::copy(&src_path, &dst_path).unwrap();
                        new_wallpaper = Some(dst_path);
                        break;
                    }
                }
            }
        }
    }

    if let Some(new_wallpaper) = new_wallpaper {
        wallpaper::set_desktop_wallpaper(new_wallpaper);
        let mut file = std::fs::File::create(spotlight_file).unwrap();
        for hash in search_set {
            file.write_all(&hash).unwrap();
        }
    }

    std::process::exit(0);
}
