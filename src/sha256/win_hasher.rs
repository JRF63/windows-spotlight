use super::{CryptHash, HASH_BUFFER_SIZE, HASH_SIZE};
use winapi::shared::*;

#[allow(dead_code)]
pub struct WinHasher {
    alg_handle: bcrypt::BCRYPT_ALG_HANDLE,
    hash_handle: bcrypt::BCRYPT_HASH_HANDLE,
    hash_data: Vec<u8>,
}

fn to_wchar(str_id: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    // create a WCHAR str and append \0 to the end
    let wchar_str: Vec<u16> = OsStr::new(str_id).encode_wide().chain(once(0)).collect();
    wchar_str
}

impl WinHasher {
    pub fn new(hash_id: &str) -> Result<WinHasher, i32> {
        let mut winhasher = WinHasher {
            alg_handle: std::ptr::null_mut(),
            hash_handle: std::ptr::null_mut(),
            hash_data: Vec::with_capacity(HASH_BUFFER_SIZE),
        };

        let mut rollback = 0;

        let mut helper_fn = || -> Result<(), i32> {
            unsafe {
                match bcrypt::BCryptOpenAlgorithmProvider(
                    &mut winhasher.alg_handle,
                    to_wchar(hash_id).as_ptr(),
                    std::ptr::null_mut(),
                    bcrypt::BCRYPT_HASH_REUSABLE_FLAG,
                ) {
                    ntstatus::STATUS_SUCCESS => rollback = 1,
                    e => return Err(e),
                };

                match bcrypt::BCryptCreateHash(
                    winhasher.alg_handle,
                    &mut winhasher.hash_handle,
                    winhasher.hash_data.as_mut_ptr(),
                    HASH_BUFFER_SIZE as _,
                    std::ptr::null_mut(),
                    0,
                    bcrypt::BCRYPT_HASH_REUSABLE_FLAG,
                ) {
                    ntstatus::STATUS_SUCCESS => rollback = 2,
                    e => return Err(e),
                };

                Ok(())
            }
        };

        match helper_fn() {
            Ok(()) => Ok(winhasher),
            Err(e) => {
                unsafe {
                    if rollback >= 1 {
                        bcrypt::BCryptCloseAlgorithmProvider(winhasher.alg_handle, 0);
                    }
                    if rollback >= 2 {
                        bcrypt::BCryptDestroyHash(winhasher.hash_handle);
                    }
                }
                Err(e)
            }
        }
    }

    pub fn update<T>(&mut self, object: &mut [T]) -> Result<(), i32> {
        unsafe {
            match bcrypt::BCryptHashData(
                self.hash_handle,
                object.as_mut_ptr() as *mut u8,
                (object.len() * std::mem::size_of::<T>()) as u32,
                0,
            ) {
                ntstatus::STATUS_SUCCESS => Ok(()),
                e => Err(e),
            }
        }
    }

    pub fn digest(&mut self) -> Result<CryptHash, i32> {
        unsafe {
            let mut result: [u8; HASH_SIZE] = std::mem::MaybeUninit::uninit().assume_init();
            match bcrypt::BCryptFinishHash(self.hash_handle, result.as_mut_ptr(), HASH_SIZE as _, 0)
            {
                ntstatus::STATUS_SUCCESS => Ok(CryptHash::new(result)),
                e => Err(e),
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size() {
        let hasher =
            WinHasher::new(bcrypt::BCRYPT_SHA256_ALGORITHM).expect("Failed to create hasher");
        let mut hash_result_size: u32 = 0;
        unsafe {
            let mut data = 0;
            let result = bcrypt::BCryptGetProperty(
                hasher.alg_handle,
                to_wchar(bcrypt::BCRYPT_HASH_LENGTH).as_ptr(),
                (&mut hash_result_size as *mut u32) as *mut u8,
                32, // size of DWORD
                &mut data,
                0,
            );
            assert!(result == ntstatus::STATUS_SUCCESS);
            assert!(hash_result_size == HASH_SIZE as u32);

            let mut hash_data_size: u32 = 0;
            let result = bcrypt::BCryptGetProperty(
                hasher.alg_handle,
                to_wchar(bcrypt::BCRYPT_OBJECT_LENGTH).as_ptr(),
                (&mut hash_data_size as *mut u32) as *mut u8,
                32, // size of DWORD
                &mut data,
                0,
            );
            assert!(result == ntstatus::STATUS_SUCCESS);
            assert!(hash_data_size == HASH_BUFFER_SIZE as u32);
        }
    }

    #[test]
    fn test_hasher() {
        let mut msg1: Vec<u8> = vec![0x61, 0x62, 0x63];
        let mut msg2: Vec<u8> = vec![0x61, 0x62, 0x63];
        let mut msg3 = String::from("hello");
        let mut hasher =
            WinHasher::new(bcrypt::BCRYPT_SHA256_ALGORITHM).expect("Failed to create hasher");

        hasher.update(&mut msg1).unwrap();
        let result = hasher.digest().unwrap();
        let hash_str = result.to_hex_str();
        assert_eq!(
            hash_str,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        println!("{}", hash_str);

        hasher.update(&mut msg2).unwrap();
        let result = hasher.digest().unwrap();
        let hash_str = result.to_hex_str();
        assert_eq!(
            hash_str,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        println!("{}", hash_str);

        unsafe {
            let mut slice = std::slice::from_raw_parts_mut(msg3.as_mut_ptr(), msg3.len());
            hasher.update(&mut slice).unwrap();
            let result = hasher.digest().unwrap();
            let hash_str = result.to_hex_str();
            assert_eq!(
                hash_str,
                "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
            );
            println!("{}", hash_str);
        }
    }
}
