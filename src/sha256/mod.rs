mod win_hasher;

pub use win_hasher::WinHasher;

pub const HASH_SIZE: usize = 32; // SHA256 is 32 bytes
const HASH_ALGORITHM: &'static str = "SHA256";

pub fn hash_fn_object() -> WinHasher {
    WinHasher::new(HASH_ALGORITHM).unwrap()
}

#[derive(PartialEq, Eq)]
pub struct CryptHash {
    data: [u8; HASH_SIZE],
}

impl CryptHash {
    pub fn new(data: [u8; HASH_SIZE]) -> Self {
        CryptHash { data }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    #[allow(dead_code)]
    pub fn to_hex_str(&self) -> String {
        self.data.iter().map(|&x| format!("{:02x}", x)).collect()
    }
}

impl std::hash::Hash for CryptHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash_slice(&self.data, state);
    }
}

pub struct CryptHasher {
    data_ptr: *const u8,
}

impl std::hash::Hasher for CryptHasher {
    fn finish(&self) -> u64 {
        // reinterprets the first 8 bytes as u64
        unsafe { *self.data_ptr.cast() }
    }

    fn write(&mut self, bytes: &[u8]) {
        // discard the len and only hash the data itself
        // of the SHA sum
        // if bytes.len() == HASH_SIZE {
        //     self.data_ptr = bytes.as_ptr();
        // }
        self.data_ptr = bytes.as_ptr();
    }
}

pub struct CryptState;

impl Default for CryptState {
    fn default() -> Self {
        CryptState {}
    }
}

impl std::hash::BuildHasher for CryptState {
    type Hasher = CryptHasher;
    fn build_hasher(&self) -> Self::Hasher {
        CryptHasher {
            data_ptr: std::ptr::null(),
        }
    }
}
