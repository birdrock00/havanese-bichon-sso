use memmap2::Mmap;
use std::fs;
use std::io;
use std::path::Path;

pub struct MboxFile {
    map: Mmap,
}

impl MboxFile {
    pub fn from_file(name: &Path) -> io::Result<Self> {
        let file = fs::File::open(name)?;
        let metadata = file.metadata()?;
        if metadata.len() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Empty MBOX file",
            ));
        }
        let map = unsafe { Mmap::map(&file)? };
        Ok(Self { map })
    }

    pub fn iter(&self) -> MboxReader<'_> {
        MboxReader::new(&self.map)
    }
}

pub struct Entry<'a> {
    pub offset: usize,
    pub data: &'a [u8],
}

pub struct MboxReader<'a> {
    data: &'a [u8],
    len: usize,
    scan_pos: usize,
    body_start: Option<usize>,
}

impl<'a> MboxReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            len: data.len(),
            scan_pos: 0,
            body_start: None,
        }
    }

    fn is_from_line(&self, i: usize) -> bool {
        if i + 5 > self.len {
            return false;
        }

        if i == 0 {
            &self.data[0..5] == b"From "
        } else {
            self.data[i - 1] == b'\n' && &self.data[i..i + 5] == b"From "
        }
    }

    fn skip_from_line(&self, mut i: usize) -> usize {
        while i < self.len && self.data[i] != b'\n' {
            i += 1;
        }
        if i < self.len {
            i += 1;
        }
        i
    }
}

impl<'a> Iterator for MboxReader<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.scan_pos < self.len {
            if self.is_from_line(self.scan_pos) {
                let from_pos = self.scan_pos;
                let body_pos = self.skip_from_line(from_pos);

                if let Some(start) = self.body_start {
                    let entry = Entry {
                        offset: start,
                        data: &self.data[start..from_pos],
                    };
                    self.body_start = Some(body_pos);
                    self.scan_pos = body_pos;
                    return Some(entry);
                } else {
                    self.body_start = Some(body_pos);
                    self.scan_pos = body_pos;
                    continue;
                }
            }

            self.scan_pos += 1;
        }
        if let Some(start) = self.body_start.take() {
            return Some(Entry {
                offset: start,
                data: &self.data[start..self.len],
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use mail_parser::MessageParser;

    use crate::modules::cli::mbox::gmail::determine_folder;

    use super::*;

    fn collect_entries(data: &[u8]) -> Vec<&[u8]> {
        let reader = MboxReader::new(data);
        reader.map(|e| e.data).collect()
    }

    #[test]
    fn two_mails() {
        let data = b"From a\nmail1\nFrom b\nmail2\n";
        let e = collect_entries(data);
        assert_eq!(e, vec![b"mail1\n", b"mail2\n"]);
    }
    #[test]
    fn no_trailing_newline() {
        let data = b"From a\nmail1";
        let e = collect_entries(data);
        assert_eq!(e, vec![b"mail1"]);
    }

    #[test]
    fn from_inside_body() {
        let data = b"From a\nhello\nFrom is here\nbye\n";
        let e = collect_entries(data);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn inline_from_not_separator() {
        let data = b"From a\nhello From world\n";
        let e = collect_entries(data);
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn realistic_mbox() {
        let data = b"From a\nH:1\n\nbody1\nFrom b\nH:2\n\nbody2\n";
        let e = collect_entries(data);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn empty_body() {
        let data = b"From a\nFrom b\nbody\n";
        let e = collect_entries(data);
        assert_eq!(e[0], b"");
        assert_eq!(e[1], b"body\n");
    }

    #[test]
    fn only_from_line() {
        let data = b"From a\n";
        let e = collect_entries(data);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0], b"");
    }

    #[test]
    fn windows_newlines() {
        let data = b"From a\r\nbody\r\nFrom b\r\nbody2\r\n";
        let e = collect_entries(data);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn many_small_mails() {
        let mut data = Vec::new();
        for i in 0..1000 {
            data.extend_from_slice(b"From a\nx\n");
        }
        let e = collect_entries(&data);
        assert_eq!(e.len(), 1000);
    }

    #[test]
    fn test11() {
        let mbox = MboxFile::from_file(Path::new("e:\\test.mbox")).unwrap();

        for e in mbox.iter() {
            let body = e.data;

            let message = MessageParser::new().parse(body).unwrap();
            let labels = message.header("X-Gmail-Labels").unwrap().as_text().unwrap();
            //println!("offset={} X-Gmail-Labels={:?}", e.offset, labels);
            println!(
                "X-Gmail-Labels={:?}, determine_folder={}",
                labels,
                determine_folder(labels)
            )


            
        }
    }
}
