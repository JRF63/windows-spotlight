extern crate winapi;

use std::io::prelude::*;
use std::collections::HashSet;

use winapi::shared::*;

// fn is_jpg(file: &mut std::fs::File) -> bool {
//     const JPG_SIG: [u8; 12] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
//     let mut buf = vec![0; 12];
//     let result = match file.read_exact(&mut buf) {
//         Ok(_) => buf == JPG_SIG,
//         Err(_) => false
//     };
//     file.seek(std::io::SeekFrom::Start(0)).expect("Failed to seek");
//     result
// }

fn is_landscape_jpg(file: &mut std::fs::File) -> bool {
    const JPG_SIG: [u8; 12] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01];
    let mut buf = vec![0; 12];
    let is_jpg = match file.read_exact(&mut buf) {
        Ok(_) => buf == JPG_SIG,
        Err(_) => false
    };
    
    if is_jpg {
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

fn hash_files(files: &[std::fs::DirEntry]) -> std::io::Result<HashSet<Vec<u8>>> {
    let mut hasher = spotlight::WinHasher::new(bcrypt::BCRYPT_SHA256_ALGORITHM).expect("Failed to create hasher");
    let mut chunk_set: HashSet<Vec<u8>> = HashSet::new();
    let mut buf = vec![0; 4096];

    for entry in files {
        let mut file = std::fs::File::open(entry.path())?;
        if is_landscape_jpg(&mut file) {
            // let mut contents = vec![];
            // file.read_to_end(&mut contents)?; // split to small reads and updates
            // hasher.update(&mut contents).expect("Failed to update hash");
            
            while let Ok(size) = file.read(&mut buf) {
                if size == 0 {
                    break;
                }
                hasher.update(&mut buf[..size]).expect("Failed to update hash");
            }
            chunk_set.insert(hasher.digest().expect("Failed to get hash digest"));

            // let hash = hasher.digest().expect("Failed to get hash digest");
            // let hash_str: String = hash.iter().map(|&i| format!("{:02x}", i)).collect();
            // println!("{:?} {}", entry.path(), hash_str);
            // chunk_set.insert(hash);
        }
    }

    Ok(chunk_set)
}

fn main() -> std::io::Result<()> {
    const CHUNK_SIZE: usize = 8;
    let spotlight_dir = std::path::Path::new(r#"C:\Users\Rafael\AppData\Local\Packages\Microsoft.Windows.ContentDeliveryManager_cw5n1h2txyewy\LocalState\Assets"#);
    let save_dir = std::path::Path::new(r#"C:\Users\Rafael\Pictures\Spotlight"#);
    // find saved file first

    let mut search_set: HashSet<Vec<u8>> = HashSet::new();

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
    let save_dir_entries = save_dir.read_dir().expect("read_dir call failed")
                                   .filter_map(is_file)
                                   .collect::<Vec<_>>();

    for chunk in save_dir_entries.chunks(CHUNK_SIZE) {
        if let Ok(chunk_set) = hash_files(chunk) {
            search_set.extend(chunk_set);
        }
    }

    // let spotlight_dir_entries = spotlight_dir.read_dir().expect("read_dir call failed")
    //                                          .filter_map(is_file)
    //                                          .collect::<Vec<_>>();

    // for chunk in spotlight_dir_entries.chunks(CHUNK_SIZE) {

    // }

    println!("Length of set {}", search_set.len());

    Ok(())
}