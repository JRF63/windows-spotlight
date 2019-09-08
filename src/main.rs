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

fn is_landscape_jpg(file: &mut std::fs::File) -> bool {
    const JPG_SIG: [u8; 12] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
    let mut buf = vec![0; 12];
    let is_jpg = match file.read_exact(&mut buf) {
        Ok(_) => buf == JPG_SIG,
        Err(_) => false
    };
    
    if is_jpg {
        // part that checks if in landscape
        file.seek(std::io::SeekFrom::Start(163)).expect("Failed to seek");
        buf.resize(4, 0);
        if let Ok(_) = file.read_exact(&mut buf) {
            let height = &buf[..2];
            let width = &buf[2..];
            if width > height {
                file.seek(std::io::SeekFrom::Start(0)).expect("Failed to seek");
                return true;
            }
        }
    }
    file.seek(std::io::SeekFrom::Start(0)).expect("Failed to seek");
    false
}

fn hash_saved_images(entries: &[std::fs::DirEntry]) -> HashSet<Vec<u8>> {
    let mut hasher = spotlight::WinHasher::new(HASH_ALGORITHM).expect("Failed to create hasher");
    let mut chunk_set: HashSet<Vec<u8>> = HashSet::new();
    let mut buf = vec![0; READ_BUF_SIZE];

    for entry in entries {
        let mut file = std::fs::File::open(entry.path()).expect("Cannot open file");
        if is_landscape_jpg(&mut file) {
            while let Ok(size) = file.read(&mut buf) {
                if size == 0 {
                    break;
                }
                hasher.update(&mut buf[..size]).expect("Failed to update hash");
            }
            chunk_set.insert(hasher.digest().expect("Failed to get hash digest"));
        }
    }

    chunk_set
}

fn find_new_image(search_set: HashSet<Vec<u8>>, entries: &[std::fs::DirEntry]) -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
    let mut hasher = spotlight::WinHasher::new(HASH_ALGORITHM).expect("Failed to create hasher");
    let mut buf = vec![0; READ_BUF_SIZE];
    let mut result = vec![];

    for entry in entries {
        let mut file = std::fs::File::open(entry.path()).expect("Cannot open file");
        if is_landscape_jpg(&mut file) {
            while let Ok(size) = file.read(&mut buf) {
                if size == 0 {
                    break;
                }
                hasher.update(&mut buf[..size]).expect("Failed to update hash");
            }
            let digest = hasher.digest().expect("Failed to get hash digest");
            if !search_set.contains(&digest) {
                let path = entry.path();
                let creation_time = entry.metadata().expect("Metadata error")
                                         .created().expect("Metadata time error");
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
        let mut copy_set: HashSet<Vec<u8>> = HashSet::new();
        copy_set.clone_from(&search_set);
        let mut chunk_images = find_new_image(copy_set, chunk);
        new_images.append(&mut chunk_images);
    }

    for (src_path, sys_time) in new_images {
        if let Ok(duration) = sys_time.duration_since(SystemTime::UNIX_EPOCH) {
            
            let nsecs = duration.as_nanos();
            let dt = chrono::Local.timestamp_nanos(nsecs as i64).naive_local();
            let date_str = dt.format("%Y%m%d").to_string();
            
            let mut dst_path = save_dir.to_path_buf();

            /* 
             * complicated suffix generation
             */
            {
                let mut char_buf: Vec<u8> = vec![];
                let mut pos: isize = -1;

                /* 
                 * Could have used loop {} but this is already
                 * ~300 million files having the same creation time
                 */
                'outer: for _ in 0..7 {
                    for i in 97u8..=122u8 {
                        char_buf.push(i);

                        // actual code
                        let mut suffix: String = date_str.clone();
                        suffix.push_str(std::str::from_utf8(&char_buf).expect(""));

                        dst_path.push(suffix);
                        dst_path.set_extension("jpg");
                        if !dst_path.is_file() {
                            std::fs::copy(src_path, dst_path).expect("Cannot copy");
                            break 'outer;
                        }
                        dst_path.pop();
                        // disgusting code past here

                        char_buf.pop();
                    }
                    
                    if pos == -1 {
                        char_buf.push(96u8);
                        pos += 1;
                    }

                    let mut cur_pos = pos;
                    loop {
                        if cur_pos < 0 {
                            char_buf.push(97u8);
                            break;
                        }
                        char_buf[cur_pos as usize] += 1;
                        if char_buf[cur_pos as usize] == 123u8 {
                            char_buf[cur_pos as usize] = 97;
                            cur_pos -= 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
}