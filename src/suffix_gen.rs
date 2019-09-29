use std::path::PathBuf;

pub struct SuffixGenerator {
    path_array: Vec<u8>,
    original_size: usize,
    extension: String,
}

impl SuffixGenerator {
    pub fn new(path: PathBuf, ext: &str) -> SuffixGenerator {
        let mut path_array: Vec<u8> = path.into_os_string().into_string().unwrap().into_bytes();
        let original_size = path_array.len() as usize;
        path_array.push(96u8);

        let mut extension = String::from(".");
        extension.push_str(ext);

        return SuffixGenerator {
            path_array,
            original_size,
            extension,
        };
    }

    fn make_pathbuf(&mut self) -> Option<PathBuf> {
        let mut tmp_string: String = String::from_utf8(self.path_array.clone()).unwrap();
        tmp_string.push_str(&self.extension);
        Some(PathBuf::from(tmp_string))
    }
}

impl Iterator for SuffixGenerator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        let mut cur_pos = self.path_array.len() - 1;
        loop {
            self.path_array[cur_pos] += 1;
            // if between 'a' and 'z'
            if self.path_array[cur_pos] <= 122u8 {
                return self.make_pathbuf();
            } else {
                // else if current char is past 'z', make it an 'a' and
                // carry over calculation to next position
                self.path_array[cur_pos] = 97;
                cur_pos -= 1;
                // not enough space, add new char
                if cur_pos < self.original_size {
                    self.path_array.push(97u8);
                    return self.make_pathbuf();
                }
            }
        }
    }
}

#[test]
fn test_suffix_gen() {
    let path_buf = PathBuf::from(r#"test-"#);
    let ext = "test";
    let mut it = SuffixGenerator::new(path_buf.clone(), ext);

    assert_eq!(Some("test-a.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-z.test"), it.nth(24).unwrap().to_str());
    assert_eq!(Some("test-aa.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-az.test"), it.nth(24).unwrap().to_str());
    assert_eq!(Some("test-ba.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-zz.test"), it.nth(648).unwrap().to_str());
    assert_eq!(Some("test-aaa.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-aba.test"), it.nth(25).unwrap().to_str());
    assert_eq!(Some("test-abb.test"), it.nth(0).unwrap().to_str());

    it = SuffixGenerator::new(path_buf.clone(), ext);
    assert_eq!(Some("test-zz.test"), it.nth(702 - 1).unwrap().to_str());

    it = SuffixGenerator::new(path_buf.clone(), ext);
    assert_eq!(Some("test-baa.test"), it.nth(1378).unwrap().to_str());
}
