//! Default Line_Break test.

use std::char;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};
use std::iter::from_fn;
use std::u32;
use unicode_linebreak::*;

const TEST_FILE: &str = "tests/LineBreakTest.txt";

#[test]
fn test_lb_default() -> io::Result<()> {
    let file = File::open(TEST_FILE)?;
    for line in BufReader::new(file)
        .lines()
        .map(|l| l.unwrap())
        .filter(|l| !l.starts_with('#'))
    {
        let (line, comment) = line.split_once("# ").expect("Missing comment");

        // Skip tests relying on some tailorable rules
        if comment.contains("[30.22]") || comment.contains("[999.0]") {
            continue;
        }

        let mut items = line.split_whitespace();
        items.next().unwrap(); // Skip first '×'
        let mut byte_idx = 0;
        let (spots, string): (Vec<_>, String) = from_fn(|| {
            if let Some(hex) = items.next() {
                let codepoint = u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .expect("Invalid codepoint");
                byte_idx += codepoint.len_utf8();

                let is_break = match items.next() {
                    Some("÷") => true,
                    Some("×") => false,
                    _ => unreachable!(),
                };

                Some(((byte_idx, is_break), codepoint))
            } else {
                None
            }
        })
        .unzip();

        let actual: Vec<_> = linebreaks(&string).map(|(i, _)| i).collect();
        let expected: Vec<_> = spots
            .into_iter()
            .filter_map(|(i, is_break)| if is_break { Some(i) } else { None })
            .collect();

        assert_eq!(
            actual, expected,
            "String: ‘{}’, comment: {}",
            string, comment
        );
    }

    Ok(())
}
