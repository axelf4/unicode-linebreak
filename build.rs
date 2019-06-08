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

fn lb_class_by_key(k: &str) -> usize {
    match k {
        "BK" => 0,
        "CR" => 1,
        "LF" => 2,
        "CM" => 3,
        "NL" => 4,
        "SG" => 5,
        "WJ" => 6,
        "ZW" => 7,
        "GL" => 8,
        "SP" => 9,
        "ZWJ" => 10,
        "B2" => 11,
        "BA" => 12,
        "BB" => 13,
        "HY" => 14,
        "CB" => 15,
        "CL" => 16,
        "CP" => 17,
        "EX" => 18,
        "IN" => 19,
        "NS" => 20,
        "OP" => 21,
        "QU" => 22,
        "IS" => 23,
        "NU" => 24,
        "PO" => 25,
        "PR" => 26,
        "SY" => 27,
        "AI" => 28,
        "AL" => 29,
        "CJ" => 30,
        "EB" => 31,
        "EM" => 32,
        "H2" => 33,
        "H3" => 34,
        "HL" => 35,
        "ID" => 36,
        "JL" => 37,
        "JV" => 38,
        "JT" => 39,
        "RI" => 40,
        "SA" => 41,
        "XX" => 42,
        _ => unreachable!(),
    }
}

static BREAK_CLASS_TABLE: [&'static str; 43] = [
    "BK", "CR", "LF", "CM", "NL", "SG", "WJ", "ZW", "GL", "SP", "ZWJ", "B2", "BA", "BB", "HY",
    "CB", "CL", "CP", "EX", "IN", "NS", "OP", "QU", "IS", "NU", "PO", "PR", "SY", "AI", "AL", "CJ",
    "EB", "EM", "H2", "H3", "HL", "ID", "JL", "JV", "JT", "RI", "SA", "XX",
];

const UNIFORM_PAGE: usize = 0x8000;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=LineBreak.txt");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tables.rs");
    let mut stream = BufWriter::new(File::create(&dest_path)?);

    stream.write_all(b"static BREAK_PROP_DATA: [[BreakClass; 256]; PAGE_COUNT] = [")?;

    let re = Regex::new(
        r"(?x)^
    (?P<start>[[:xdigit:]]{4,}) # Unicode code point
    (?:\.{2}(?P<end>[[:xdigit:]]{4,}))? # End range
    ;
    (?P<lb>\w{2,3}) # Line_Break property",
    )
    .unwrap();

    let mut last = 0;
    let mut values = BufReader::new(File::open("LineBreak.txt")?)
        .lines()
        .map(|l| l.unwrap())
        .filter(|l| !(l.starts_with('#') || l.is_empty()))
        .flat_map(|l| {
            let caps = re.captures(&l).unwrap();
            let start = u32::from_str_radix(&caps["start"], 16).unwrap();
            let end = caps
                .name("end")
                .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
                .unwrap_or(start);
            let lb = lb_class_by_key(&caps["lb"]);

            let iter = (last..=end).into_iter().map(move |code| {
                if code < start {
                    lb_class_by_key(default_value(code))
                } else {
                    lb
                }
            });
            last = end + 1;
            iter
        });

    let mut page = Vec::new();
    let mut page_count = 0;
    let mut page_indices = Vec::new();
    let mut should_continue = true;
    while should_continue {
        for _ in 0..256 {
            match values.next() {
                Some(value) => page.push(value),
                None => {
                    should_continue = false;
                    break;
                }
            }
        }

        if let Some(first) = page.first() {
            let page_equal = page.iter().all(|v| v == first);
            page_indices.push(if page_equal {
                first | UNIFORM_PAGE
            } else {
                writeln!(
                    stream,
                    "[{}],",
                    page.iter()
                        .map(|&v| v)
                        .chain((page.len()..256).into_iter().map(|_| lb_class_by_key("XX")))
                        .map(|v| BREAK_CLASS_TABLE[v])
                        .collect::<Vec<_>>()
                        .join(",")
                )?;
                page_count += 1;
                page_count - 1
            });

            // Reset values to default
            page.clear();
        }
    }

    writeln!(
        stream,
        r"];

        const PAGE_COUNT: usize = {};
        const UNIFORM_PAGE: usize = 0x8000;
        static PAGE_INDICES: [usize; {}] = [",
        page_count,
        page_indices.len()
    )?;
    for page_idx in page_indices {
        write!(stream, "{},", page_idx)?;
    }
    writeln!(stream, "];")?;

    Ok(())
}
