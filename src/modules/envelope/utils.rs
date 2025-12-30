use mail_parser::parsers::MessageStream;
use regex::{Captures, Regex};

fn merge_contiguous_encoded_words(input: &str) -> String {
    let block_re =
        Regex::new(r"(?:=\?[^?]+\?[bBqQ]\?[^?]+\?=)(?:\s+(?:=\?[^?]+\?[bBqQ]\?[^?]+\?=))+")
            .unwrap();

    let word_re = Regex::new(r"=\?([^?]+)\?([bBqQ])\?([^?]+)\?=").unwrap();

    block_re
        .replace_all(input, |caps: &Captures| {
            let whole = caps.get(0).unwrap().as_str();

            let mut charset: Option<String> = None;
            let mut encoding: Option<String> = None;
            let mut combined = String::new();
            let mut ok = true;

            for cap in word_re.captures_iter(whole) {
                let cs = &cap[1];
                let enc = cap[2].to_ascii_uppercase();
                let text = &cap[3];

                if let Some(ref c) = charset {
                    if c != cs {
                        ok = false;
                        break;
                    }
                } else {
                    charset = Some(cs.to_string());
                }

                if let Some(ref e) = encoding {
                    if e != &enc {
                        ok = false;
                        break;
                    }
                } else {
                    encoding = Some(enc);
                }

                combined.push_str(text);
            }

            if ok {
                format!(
                    "=?{}?{}?{}?=",
                    charset.unwrap(),
                    encoding.unwrap(),
                    combined
                )
            } else {
                whole.to_string()
            }
        })
        .to_string()
}

pub fn normalize_subject(raw_subject: Option<&str>) -> String {
    let subject = match raw_subject {
        Some(subject) => merge_contiguous_encoded_words(subject),
        None => return String::new(),
    };

    MessageStream::new(subject.as_bytes())
        .parse_unstructured()
        .as_text()
        .map(String::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::modules::envelope::utils::merge_contiguous_encoded_words;


    #[tokio::test]
    async fn test3() {
        let s = "Hello =?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?= !!!";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "Hello =?UTF-8?B?SGVsbG8=V29ybGQ=?= !!!"
        );

        let s = "=?UTF-8?B?QQ==?= =?UTF-8?B?Qg==?= =?UTF-8?B?Qw==?=";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "=?UTF-8?B?QQ==Qg==Qw==?="
        );

        let s = "=?UTF-8?B?QQ==?= =?UTF-8?B?Qg==?= test =?UTF-8?B?Qw==?= =?UTF-8?B?RA==?=";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "=?UTF-8?B?QQ==Qg==?= test =?UTF-8?B?Qw==RA==?="
        );

        let s = "=?UTF-8?B?QQ==?= =?GBK?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), s);
        let s = "=?UTF-8?B?QQ==?= =?UTF-8?Q?Qg?=";
        assert_eq!(merge_contiguous_encoded_words(s), s);

        let s = "=?UTF-8?b?QQ==?= =?UTF-8?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), "=?UTF-8?B?QQ==Qg==?=");
        let s = "Hello =?UTF-8?B?SGVsbG8=?= !!!";
        assert_eq!(merge_contiguous_encoded_words(s), s);
        let s = "=?UTF-8?B?QQ==?=    =?UTF-8?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), "=?UTF-8?B?QQ==Qg==?=");
        let s = "Just a normal subject line";
        assert_eq!(merge_contiguous_encoded_words(s), s);
        let s = "=?UTF-8?Q?Hello_?= =?UTF-8?Q?World?=";
        assert_eq!(merge_contiguous_encoded_words(s), "=?UTF-8?Q?Hello_World?=");
    }
}
