extern crate winapi;

use winapi::shared::*;

#[allow(dead_code)]
struct WinHasher {
    alg_handle: bcrypt::BCRYPT_ALG_HANDLE,
    hash_handle: bcrypt::BCRYPT_HASH_HANDLE,
    hash_data: Vec<u8>,
    hash_result: Vec<u8>,
    hash_result_size: u32
}

impl WinHasher {
    pub fn new(hash_id: &str) -> Result<WinHasher, i32> {

        fn to_wchar(str_id: &str) -> Vec<u16> {
            use std::ffi::OsStr;
            use std::iter::once;
            use std::os::windows::ffi::OsStrExt;
            // create an WCHAR str and append \0 to the end
            let wchar_str: Vec<u16> = OsStr::new(str_id).encode_wide().chain(once(0)).collect();
            wchar_str
        }

        unsafe fn inner_fn(hash_id: &str) -> Result<WinHasher, i32> {
            let mut alg_handle = std::ptr::null_mut();
            let mut hash_handle = std::ptr::null_mut();
            let mut hash_data: Vec<u8> = vec![];
            let mut hash_result: Vec<u8>  = vec![];
            let mut hash_result_size: u32 = 0;

            let mut rollback = 0;

            let mut helper_fn = || -> Result<(), i32> {
                
                let mut status;

                status = bcrypt::BCryptOpenAlgorithmProvider(
                    &mut alg_handle,
                    to_wchar(hash_id).as_ptr(),
                    std::ptr::null_mut(),
                    bcrypt::BCRYPT_HASH_REUSABLE_FLAG);
                match status {
                    ntstatus::STATUS_SUCCESS => rollback += 1,
                    _ => return Err(status)
                };

                let mut data1 = 0;
                let mut hash_data_size: u32 = 0;
                status = bcrypt::BCryptGetProperty(
                    alg_handle,
                    to_wchar(bcrypt::BCRYPT_OBJECT_LENGTH).as_ptr(),
                    (&mut hash_data_size as *mut u32) as *mut u8,
                    32, // size of DWORD
                    &mut data1,
                    0);
                match status {
                    ntstatus::STATUS_SUCCESS => (),
                    _ => return Err(status)
                };
                
                hash_data = Vec::with_capacity(hash_data_size as usize);
                hash_data.set_len(hash_data_size as usize);

                status = bcrypt::BCryptCreateHash(
                    alg_handle,
                    &mut hash_handle,
                    hash_data.as_mut_ptr(),
                    hash_data_size.into(),
                    std::ptr::null_mut(),
                    0,
                    0);
                match status {
                    ntstatus::STATUS_SUCCESS => rollback += 1,
                    _ => return Err(status)
                };

                let mut data2 = 0;
                status = bcrypt::BCryptGetProperty(
                    alg_handle,
                    to_wchar(bcrypt::BCRYPT_HASH_LENGTH).as_ptr(),
                    (&mut hash_result_size as *mut u32) as *mut u8,
                    32, // size of DWORD
                    &mut data2,
                    0);
                match status {
                    ntstatus::STATUS_SUCCESS => (),
                    _ => return Err(status)
                };
                hash_result = Vec::with_capacity(hash_result_size as usize);
                hash_result.set_len(hash_result_size as usize);
                Ok(())
            };

            match helper_fn() {
                Ok(()) => {
                    Ok(WinHasher {
                        alg_handle,
                        hash_handle,
                        hash_data,
                        hash_result,
                        hash_result_size
                    })
                }
                Err(status) => {
                    if rollback >= 1 {
                        bcrypt::BCryptCloseAlgorithmProvider(alg_handle, 0);
                    }
                    if rollback >= 2 {
                        bcrypt::BCryptDestroyHash(hash_handle);
                    }
                    Err(status)
                }
            }
        }

        let result = unsafe { inner_fn(hash_id) };
        result
    }

    pub fn hash_object(&mut self, object: &mut [u8]) -> Result<Vec<u8>, i32> {
        unsafe fn inner_fn(hasher: &mut WinHasher, object: &mut [u8]) -> Result<(), i32> {
            let mut status;
            status = bcrypt::BCryptHashData(
                hasher.hash_handle,
                object.as_mut_ptr(),
                object.len() as u32,
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };

            status = bcrypt::BCryptFinishHash(
                hasher.hash_handle,
                hasher.hash_result.as_mut_ptr(),
                hasher.hash_result_size.into(),
                0);
            match status {
                ntstatus::STATUS_SUCCESS => (),
                _ => return Err(status)
            };
            Ok(())
        }

        let result = unsafe { (inner_fn(self, object)) };
        match result {
            Ok(_) => {
                let duplicate = self.hash_result.clone();
                Ok(duplicate)
            },
            Err(e) => Err(e)
        }
    }
}

impl Drop for WinHasher {
    fn drop(&mut self) {
        unsafe {
            bcrypt::BCryptCloseAlgorithmProvider(self.alg_handle, 0);
            bcrypt::BCryptDestroyHash(self.hash_handle);
        }
    }
}

fn main() {
    let mut msg = vec![0x61, 0x62, 0x63];
    if let Ok(mut hasher) = WinHasher::new(bcrypt::BCRYPT_SHA256_ALGORITHM) {
        if let Ok(result) = hasher.hash_object(&mut msg) {
            for i in result {
                print!{"{:x}", i}
            }
        }
    };
}