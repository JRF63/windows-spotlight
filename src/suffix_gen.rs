use std::path::PathBuf;

pub struct SuffixGenerator {
    path_array: Vec<u8>,
    original_size: isize,
    extension: String,
    array_pos: isize,
    ascii_lower: Box<dyn Iterator<Item=u8>>
}

impl SuffixGenerator {
    // use OsString
    pub fn new(path: &PathBuf, ext: &str) -> SuffixGenerator {
        let mut path_str: String = path.to_str().expect("Invalid unicode").to_string();
        let path_array: Vec<u8> = unsafe { path_str.as_bytes_mut().to_vec() };
        let original_size = path_array.len() as isize;
        let mut extension = String::from(".");
        extension.push_str(ext);
        let ascii_iter = Box::new((97u8..=122u8).chain(std::iter::once(0u8)).cycle());
        SuffixGenerator {
            path_array,
            original_size,
            extension,
            array_pos: -1,
            ascii_lower: ascii_iter
        }
    }

    fn adjust_suffix(&mut self) {
        // place in ::new
        if self.array_pos == -1 {
            self.path_array.push(96u8);
            self.array_pos = 0;
        }

        let mut cur_pos = self.array_pos;
        loop {
            if cur_pos < 0 {
                self.path_array.push(97u8);
                self.array_pos += 1;
                break;
            }
            let pos_offset = (cur_pos + self.original_size) as usize;
            self.path_array[pos_offset] += 1;
            if self.path_array[pos_offset] == 123u8 {
                self.path_array[pos_offset] = 97;
                cur_pos -= 1;
            } else {
                break;
            }
        }
    }
}

impl Iterator for SuffixGenerator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.ascii_lower.next() {
                Some(0) => {
                    self.adjust_suffix();
                },
                Some(ch) => {
                    self.path_array.push(ch);
                    
                    let mut tmp_str: String = String::from_utf8(self.path_array.clone()).expect("Invalid unicode");
                    tmp_str.push_str(&self.extension);
                    let result = PathBuf::from(tmp_str);

                    self.path_array.pop(); // remove ch
                    return Some(result);
                },
                None => panic!("Should be unreachable")
            }
        }
    }
}

#[test]
fn test_suffix_gen() {
    let path_buf = PathBuf::from(r#"test-"#);
    let ext = "test";
    let mut it = SuffixGenerator::new(&path_buf, ext);
    
    assert_eq!(Some("test-a.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-z.test"), it.nth(24).unwrap().to_str());
    assert_eq!(Some("test-aa.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-az.test"), it.nth(24).unwrap().to_str());
    assert_eq!(Some("test-ba.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-zz.test"), it.nth(648).unwrap().to_str());
    assert_eq!(Some("test-aaa.test"), it.nth(0).unwrap().to_str());
    assert_eq!(Some("test-aba.test"), it.nth(25).unwrap().to_str());
    assert_eq!(Some("test-abb.test"), it.nth(0).unwrap().to_str());
    
    it = SuffixGenerator::new(&path_buf, ext);
    assert_eq!(Some("test-zz.test"), it.nth(702 - 1).unwrap().to_str());

    it = SuffixGenerator::new(&path_buf, ext);
    assert_eq!(Some("test-baa.test"), it.nth(1378).unwrap().to_str());
}