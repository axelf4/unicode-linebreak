use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;

fn default_value(codepoint: u32) -> &'static str {
    match codepoint {
        // The unassigned code points in the following blocks default to "ID"
        0x3400...0x4DBF | 0x4E00...0x9FFF | 0xF900...0xFAFF => "ID",
        // All undesignated code points in Planes 2 and 3, whether inside or outside of allocated blocks, default to "ID"
        0x20000...0x2FFFD | 0x30000...0x3FFFD => "ID",
        // All unassigned code points in the following Plane 1 range, whether inside or outside of allocated blocks, also default to "ID"
        0x1F000...0x1FFFD => "ID",
        // The unassigned code points in the following block default to "PR"
        0x20A0...0x20CF => "PR",
        // All code points, assigned and unassigned, that are not listed explicitly are given the value "XX"
        _ => "XX",
    }
}

fn lb_property_by_key(k: &str) -> usize {
    match k {
        "BK" => 0,
        "CR" => 1,
        "LF" => 2,
        "CM" => 3,
        "SG" => 4,
        "ZWS" => 5,
        "IN" => 6,
        "GL" => 7,
        "CB" => 8,
        "SP" => 9,
        "BA" => 10,
        "BB" => 11,
        "B2" => 12,
        "HY" => 13,
        "NS" => 14,
        "OP" => 15,
        "CL" => 16,
        "QU" => 17,
        "EX" => 18,
        "ID" => 19,
        "NU" => 20,
        "IS" => 21,
        "SY" => 22,
        "AL" => 23,
        "PR" => 24,
        "PO" => 25,
        "SA" => 26,
        "AI" => 27,
        "XX" => 28,
        "NL" => 29,
        "WJ" => 30,
        "JL" => 31,
        "JV" => 32,
        "JT" => 33,
        "H2" => 34,
        "H3" => 35,
        "CP" => 36,
        "CJ" => 37,
        "HL" => 38,
        "RI" => 39,
        "EB" => 40,
        "EM" => 41,
        "ZWJ" => 42,
        _ => unreachable!(),
    }
}

const UNIFORM_PAGE: usize = 0x8000;

fn main() -> std::io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tables.rs");
    let mut stream = BufWriter::new(File::create(&dest_path)?);

    stream.write_all(b"static BREAK_PROP_DATA: [[BreakClass; 256]; PAGE_COUNT] = [")?;

    let re = Regex::new(
        r"(?x)^
    (?P<start>[[:xdigit:]]{4,}) # Unicode code point
    (?:\.{2}(?P<end>[[:xdigit:]]{4,}))? # End range
    ;
    (?P<lb>\w{2,3}) # Line_Break property
        ",
    )
    .unwrap();

    let data = File::open("LineBreak.txt")?;

    let mut page = String::new();
    let mut page_length = 0;
    let mut page_equal = true;
    let mut page_first = String::new();
    let mut page_count = 0;
    let mut page_indices = Vec::new();

    let mut last = -1;
    for line in BufReader::new(data).lines() {
        let line = line?;

        if line.starts_with("#") || line.is_empty() {
            continue;
        }

        let caps = re.captures(&line).unwrap();

        let start = u32::from_str_radix(&caps["start"], 16).unwrap();
        let end = caps
            .name("end")
            .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
            .unwrap_or(start);
        let lb = &caps["lb"];

        for code in (last + 1) as u32..=end {
            let value = if code < start {
                default_value(code)
            } else {
                lb
            };

            if page_length == 0 {
                page_first = value.to_owned();
            } else {
                if page_equal && value != page_first {
                    page_equal = false;
                }
            }

            page.push_str(value);
            page.push(',');
            page_length += 1;

            if page_length == 256 {
                page_indices.push(if page_equal {
                    lb_property_by_key(value) | UNIFORM_PAGE
                } else {
                    writeln!(stream, "[ {} ],", page)?;
                    page_count += 1;
                    page_count - 1
                });

                // Reset values to default
                page.clear();
                page_equal = true;
                page_length = 0;
            }
        }
        last = end as i32;
    }

    writeln!(
        stream,
        r"];

    const PAGE_COUNT: usize = {};

    const UNIFORM_PAGE: usize = 0x8000;
    static PAGE_INDICES: [usize; {}] = [
    ",
        page_count,
        page_indices.len()
    )?;

    for page_idx in page_indices {
        write!(stream, "{},", page_idx)?;
    }

    writeln!(stream, "];")?;

    Ok(())
}
