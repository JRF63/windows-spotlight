extern crate winapi;

use std::mem;
    
use winapi::shared::*;

struct WinHasher {
    alg_handle: *mut bcrypt::BCRYPT_ALG_HANDLE,
    hash_handle: *mut bcrypt::BCRYPT_HASH_HANDLE
}

fn to_wchar(str_id: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    // create an WCHAR str and append \0 to the end
    let wchar_str: Vec<u16> = OsStr::new(str_id).encode_wide().chain(once(0)).collect();
    wchar_str
}

fn create_hash() {
    unsafe fn init_hash() {
        let mut rollback = 0;
        let mut alg_handle: *mut winapi::ctypes::c_void = std::ptr::null_mut();
        let mut hash_handle = std::ptr::null_mut();
        let mut hash: Vec<u8> = vec![];
        let mut hash_obj: Vec<u8>  = vec![];
        let mut msg = vec![0x61, 0x62, 0x63];

        let mut inner_func = || -> Result<(), i32> {
            let mut status;
            status = bcrypt::BCryptOpenAlgorithmProvider(
                &mut alg_handle,
                to_wchar(bcrypt::BCRYPT_SHA256_ALGORITHM).as_ptr(),
                std::ptr::null_mut(),
                0);
            match status {
                ntstatus::STATUS_SUCCESS => rollback += 1,
                _ => return Err(status)
            };

            let mut data1 = 0;
            let mut hash_obj_size: u32 = 0;
            status = bcrypt::BCryptGetProperty(
                alg_handle,
                to_wchar(bcrypt::BCRYPT_OBJECT_LENGTH).as_ptr(),
                (&mut hash_obj_size as *mut u32) as *mut u8,
                mem::size_of::<u32>() as u32,
                &mut data1,
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };

            hash_obj = Vec::with_capacity(hash_obj_size as usize);
            // hash_obj.set_len(hash_obj_size as usize);

            let mut data2 = 0;
            let mut hash_size: u32 = 0;
            status = bcrypt::BCryptGetProperty(
                alg_handle,
                to_wchar(bcrypt::BCRYPT_HASH_LENGTH).as_ptr(),
                (&mut hash_size as *mut u32) as *mut u8,
                mem::size_of::<u32>() as u32,
                &mut data2,
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };

            hash = Vec::with_capacity(hash_size as usize);
            // hash.set_len(hash_size as usize);

            status = bcrypt::BCryptCreateHash(
                alg_handle,
                &mut hash_handle,
                hash_obj.as_mut_ptr(),
                hash_obj_size.into(),
                std::ptr::null_mut(),
                0,
                0);
            match status {
                ntstatus::STATUS_SUCCESS => rollback += 1,
                ntstatus::STATUS_BUFFER_TOO_SMALL => println!("STATUS_BUFFER_TOO_SMALL"),
                _ => {println!("Create"); return Err(status)}
            };

            status = bcrypt::BCryptHashData(
                hash_handle,
                msg.as_mut_ptr(),
                (msg.len() * mem::size_of::<u8>()) as u32,
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };

            status = bcrypt::BCryptFinishHash(
                hash_handle,
                hash.as_mut_ptr(),
                hash_size.into(),
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };

            Ok(())
        };
        
        match inner_func() {
            Ok(()) => println!("Success"),
            Err(status) => println!("Status {}", status)
        }

        if rollback >= 1 {
            bcrypt::BCryptCloseAlgorithmProvider(alg_handle, 0);
        }
        if rollback >= 2 {
            bcrypt::BCryptDestroyHash(hash_handle);
        }
    }
    unsafe {
        init_hash();
    }
}

fn main() {
    create_hash();
}