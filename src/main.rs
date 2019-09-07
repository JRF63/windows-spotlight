extern crate winapi;

use winapi::shared::*;

#[allow(dead_code)]
struct WinHasher {
    alg_handle: bcrypt::BCRYPT_ALG_HANDLE,
    hash_handle: bcrypt::BCRYPT_HASH_HANDLE,
    hash_data: Vec<u8>,
    hash_result: Vec<u8>
}

impl WinHasher {
    pub fn new(hash_id: &str) -> Result<WinHasher, i32> {

        fn to_wchar(str_id: &str) -> Vec<u16> {
            use std::ffi::OsStr;
            use std::iter::once;
            use std::os::windows::ffi::OsStrExt;
            // create a WCHAR str and append \0 to the end
            let wchar_str: Vec<u16> = OsStr::new(str_id).encode_wide().chain(once(0)).collect();
            wchar_str
        }

        let mut alg_handle = std::ptr::null_mut();
        let mut hash_handle = std::ptr::null_mut();
        let mut hash_data: Vec<u8> = vec![];
        let mut hash_result: Vec<u8>  = vec![];

        let mut rollback = 0;

        let mut helper_fn = || -> Result<(), i32> {
            let mut data = 0;

            match unsafe { bcrypt::BCryptOpenAlgorithmProvider(
                &mut alg_handle,
                to_wchar(hash_id).as_ptr(),
                std::ptr::null_mut(),
                bcrypt::BCRYPT_HASH_REUSABLE_FLAG)
            } {
                ntstatus::STATUS_SUCCESS => rollback += 1,
                e => return Err(e)
            };

            let mut hash_data_size: u32 = 0;
            match unsafe { bcrypt::BCryptGetProperty(
                alg_handle,
                to_wchar(bcrypt::BCRYPT_OBJECT_LENGTH).as_ptr(),
                (&mut hash_data_size as *mut u32) as *mut u8,
                32, // size of DWORD
                &mut data,
                0)
            } {
                ntstatus::STATUS_SUCCESS => (),
                e => return Err(e)
            };
            hash_data.reserve_exact(hash_data_size as usize);
            unsafe{ hash_data.set_len(hash_data_size as usize) };

            match unsafe { bcrypt::BCryptCreateHash(
                alg_handle,
                &mut hash_handle,
                hash_data.as_mut_ptr(),
                hash_data_size.into(),
                std::ptr::null_mut(),
                0,
                0)
            } {
                ntstatus::STATUS_SUCCESS => rollback += 1,
                e => return Err(e)
            };

            let mut hash_result_size: u32 = 0;
            match unsafe { bcrypt::BCryptGetProperty(
                alg_handle,
                to_wchar(bcrypt::BCRYPT_HASH_LENGTH).as_ptr(),
                (&mut hash_result_size as *mut u32) as *mut u8,
                32, // size of DWORD
                &mut data,
                0)
            } {
                ntstatus::STATUS_SUCCESS => (),
                e => return Err(e)
            };
            hash_result.reserve_exact(hash_result_size as usize);
            unsafe{ hash_result.set_len(hash_result_size as usize) };

            Ok(())
        };

        match helper_fn() {
            Ok(()) => {
                Ok(WinHasher {
                    alg_handle,
                    hash_handle,
                    hash_data,
                    hash_result
                })
            }
            Err(e) => {
                unsafe {
                    if rollback >= 1 {
                        bcrypt::BCryptCloseAlgorithmProvider(alg_handle, 0);
                    }
                    if rollback >= 2 {
                        bcrypt::BCryptDestroyHash(hash_handle);
                    }
                }
                Err(e)
            }
        }
    }

    pub fn hash_object<T>(&mut self, object: &mut [T]) -> Result<Vec<u8>, i32> {
        let mut inner_fn = || -> Result<(), i32> {
            match unsafe { bcrypt::BCryptHashData(
                self.hash_handle,
                object.as_mut_ptr() as *mut u8,
                (object.len() * std::mem::size_of::<T>()) as u32,
                0)
            } {
                ntstatus::STATUS_SUCCESS => (),
                e => return Err(e)
            };

            match unsafe { bcrypt::BCryptFinishHash(
                self.hash_handle,
                self.hash_result.as_mut_ptr(),
                self.hash_result.len() as u32,
                0)
            } {
                ntstatus::STATUS_SUCCESS => (),
                e => return Err(e)
            };

            Ok(())
        };

        match inner_fn() {
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
    let mut msg1: Vec<u8> = vec![0x61, 0x62, 0x63];
    let mut msg2: Vec<u8> = vec![0x61, 0x62, 0x63];
    let mut msg3 = String::from("hello");
    let mut hasher = WinHasher::new(bcrypt::BCRYPT_SHA256_ALGORITHM).expect("Failed to create hasher");
    if let Ok(result) = hasher.hash_object(&mut msg1) {
        for i in result {
            print!{"{:02x}", i}
        }
        println!("");
    }
    if let Ok(result) = hasher.hash_object(&mut msg2) {
        for i in result {
            print!{"{:02x}", i}
        }
        println!("");
    }
    
    unsafe {
        let mut slice = std::slice::from_raw_parts_mut(msg3.as_mut_ptr(), msg3.len());
        if let Ok(result) = hasher.hash_object(&mut slice) {
            for i in result {
                print!{"{:02x}", i}
            }
            println!("");
        }
    }
}