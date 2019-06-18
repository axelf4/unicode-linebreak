#![recursion_limit = "512"]

use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;

fn default_value(codepoint: u32) -> &'static str {
    match codepoint {
        // The unassigned code points in the following blocks default to "ID"
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => "ID",
        // All undesignated code points in Planes 2 and 3, whether inside or outside of allocated blocks, default to "ID"
        0x20000..=0x2FFFD | 0x30000..=0x3FFFD => "ID",
        // All unassigned code points in the following Plane 1 range, whether inside or outside of allocated blocks, also default to "ID"
        0x1F000..=0x1FFFD => "ID",
        // The unassigned code points in the following block default to "PR"
        0x20A0..=0x20CF => "PR",
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

const NUM_CLASSES: usize = 43;
const UNIFORM_PAGE: usize = 0x8000;
static BREAK_CLASS_TABLE: [&'static str; NUM_CLASSES] = [
    "BK", "CR", "LF", "CM", "NL", "SG", "WJ", "ZW", "GL", "SP", "ZWJ", "B2", "BA", "BB", "HY",
    "CB", "CL", "CP", "EX", "IN", "NS", "OP", "QU", "IS", "NU", "PO", "PR", "SY", "AI", "AL", "CJ",
    "EB", "EM", "H2", "H3", "HL", "ID", "JL", "JV", "JT", "RI", "SA", "XX",
];

// We want to parse the rules into a state machine using a pair table. Each value in the table
// specifies the next state and whether its an forced/allowed break. To handles rules such as
//
// B SP* ÷ A
//
// the extra state BSP is employed in the pair table friendly equivalent rule
//
// (B | BSP) ÷ A, Treat (B | BSP) SP* as if it were BSP, Treat BSP as if it were SP

const ALLOWED_BREAK_BIT: u8 = 0x80;
const MANDATORY_BREAK_BIT: u8 = 0x40;
const NUM_STATES: usize = NUM_CLASSES + 1; // Includes eot
const NUM_EXTRA_STATES: usize = 10;
static EXTRA_STATES: [&'static str; NUM_EXTRA_STATES] = [
    "eot", "sot", "ZWSP", "OPSP", "QUSP", "CLSP", "CPSP", "B2SP", "HLHYBA", "RIRI",
];

/// Returns a pair table conforming to the specified rules.
///
/// The rule syntax is a modified subset of the one in Unicode Standard Annex #14.
macro_rules! rules2pair_table {
    ($pair_table:ident $map:ident $($tt:tt)+) => {
        rules2pair_table! {@rule $pair_table.len(), ($map $pair_table) $($tt)+}
    };

    (@rule $len:expr, ($($args:tt)*) Treat X $($tt:tt)*) => {
        rules2pair_table! {@rule NUM_STATES, ($($args)* treat_x) $($tt)*}
    };
    (@rule $len:expr, ($map:ident $pair_table:ident $($args:tt)*) Treat $($tt:tt)*) => {
        rules2pair_table! {@rule $pair_table.len(), ($map $pair_table $($args)* treat) $($tt)*}
    };
    (@rule $len:expr, ($map:ident $pair_table:ident $($args:tt)*) * as if it were X where X = $($tt:tt)*) => {
        rules2pair_table! {@rule $pair_table.len(), ($map $pair_table $($args)* as_if_it_were_x_where_x_is) $($tt)*}
    };

    (@rule $len:expr, ($map:ident $pair_table:ident treat_x [$second:expr] as_if_it_were_x_where_x_is [$X:expr]) $(, $($tt:tt)*)?) => {
        $(rules2pair_table! {$pair_table $map $($tt)*})?

        for i in $X {
            for &j in &$second {
                $pair_table[i][j] = i as u8;
            }
        }
    };
    (@rule $len:expr, ($map:ident $pair_table:ident treat [$first:expr] [$second:expr]) as if it were $cls:ident $(, $($tt:tt)*)?) => {
        $(rules2pair_table! {$pair_table $map $($tt)*})?

        let cls = $map(stringify!($cls)) as u8;
        for i in $first {
            for &j in &$second {
                $pair_table[i][j] = cls;
            }
        }
    };
    (@rule $len:expr, ($map:ident $pair_table:ident treat [$first:expr]) as if it were $cls:ident $(, $($tt:tt)*)?) => {
        $(rules2pair_table! {$pair_table $map $($tt)*})?

        let xidx = $map(stringify!($cls));
        for &j in &$first {
            if xidx < NUM_STATES && j < NUM_STATES {
                for i in 0..$pair_table.len() {
                    $pair_table[i][j] = $pair_table[i][xidx];
                }
            }
        }
        let row = $pair_table[xidx].clone();
        for i in $first {
            $pair_table[i].copy_from_slice(&row);
        }
    };

    (@rule $len:expr, ($($args:tt)*) ALL $($tt:tt)*) => {
        let indices = (0..$len).into_iter().collect::<Vec<_>>();
        rules2pair_table! {@rule NUM_STATES, ($($args)* [indices]) $($tt)*}
    };
    // Single class pattern
    (@rule $len:expr, ($map:ident $($args:tt)*) $cls:ident $($tt:tt)*) => {
        let indices = vec![$map(stringify!($cls))];
        rules2pair_table! {@rule NUM_STATES, ($map $($args)* [indices]) $($tt)*}
    };
    // Parse (X | ...) patterns
    (@rule $len:expr, ($map:ident $($args:tt)*) ($($cls:ident)|+) $($tt:tt)*) => {
        let indices = vec![$($map(stringify!($cls))),+];
        rules2pair_table! {@rule NUM_STATES, ($map $($args)* [indices]) $($tt)*}
    };
    // Parse [^ ...] patterns
    (@rule $len:expr, ($map:ident $($args:tt)*) [^$($cls:ident)+] $($tt:tt)*) => {
        let excluded = vec![$($map(stringify!($cls))),+];
        let indices = (0..$len).into_iter().filter(|i| !excluded.contains(i)).collect::<Vec<_>>();
        rules2pair_table! {@rule NUM_STATES, ($map $($args)* [indices]) $($tt)*}
    };
    // Operators
    (@rule $len:expr, ($($args:tt)*) '÷' $($tt:tt)+) => {rules2pair_table! {@rule NUM_STATES, ($($args)* '÷') $($tt)+}};
    (@rule $len:expr, ($($args:tt)*) '×' $($tt:tt)+) => {rules2pair_table! {@rule NUM_STATES, ($($args)* '×') $($tt)+}};
    (@rule $len:expr, ($($args:tt)*) '!' $($tt:tt)+) => {rules2pair_table! {@rule NUM_STATES, ($($args)* '!') $($tt)+}};

    (@rule $len:expr, ($map:ident $pair_table:ident $([$first:expr])? $operator:literal $([$second:expr])?) $(, $($tt:tt)*)?) => {
        $(rules2pair_table! {$pair_table $map $($tt)*})?

        #[allow(unused)]
        let first = 0..$pair_table.len(); // Default to ALL
        $(let first = $first;)?
        #[allow(unused)]
        let second = (0..NUM_STATES).into_iter().collect::<Vec<_>>(); // Default to ALL
        $(let second = $second;)?
        for i in first {
            for &j in &second {
                match $operator {
                    '!' => $pair_table[i][j] |= ALLOWED_BREAK_BIT | MANDATORY_BREAK_BIT,
                    '÷' => $pair_table[i][j] |= ALLOWED_BREAK_BIT,
                    '×' => $pair_table[i][j] = $pair_table[i][j] & !(ALLOWED_BREAK_BIT | MANDATORY_BREAK_BIT),
                    _ => unreachable!("Bad operator"),

                }
            }
        }
    };

    ($pair_table:ident $map:ident) => {}; // Exit condition
    // Entry point
    ($len:expr, $map:ident; $($tt:tt)+) => {{
        let mut pair_table = [[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43]; $len];
        rules2pair_table!{pair_table $map $($tt)+}
        pair_table
    }};
}

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=LineBreak.txt");

    fn state_by_key(k: &'static str) -> usize {
        if let Some(i) = EXTRA_STATES.iter().position(|&s| s == k) {
            NUM_CLASSES + i
        } else {
            lb_class_by_key(k)
        }
    }

    assert!(NUM_CLASSES + NUM_EXTRA_STATES <= 0x3F, "Too many states");

    let pair_table = rules2pair_table! {
        NUM_CLASSES + NUM_EXTRA_STATES, state_by_key;

        // Non-tailorable Line Breaking Rules
        // LB1 Assign a line breaking class to each code point of the input. Resolve AI, CB, CJ,
        // SA, SG, and XX into other line breaking classes depending on criteria outside the scope
        // of this algorithm.
        Treat (AI | SG | XX | SA) as if it were AL, Treat CJ as if it were NS,
        // Start and end of text:
        sot '×', // LB2 Never break at the start of text.
        '!' eot, // LB3 Always break at the end of text.
        // Mandatory breaks:
        BK '!', // LB4 Always break after hard line breaks.
        // LB5 Treat CR followed by LF, as well as CR, LF, and NL as hard line breaks.
        CR '×' LF, CR '!', LF '!', NL '!',
        '×' (BK | CR | LF | NL), // LB6 Do not break before hard line breaks.
        // Explicit breaks and non-breaks:
        '×' SP, '×' ZW, // LB7 Do not break before spaces or zero width space.
        // LB8 Break before any character following a zero-width space, even if one or more spaces
        // intervene.
        (ZW | ZWSP) '÷', Treat (ZW | ZWSP) SP as if it were ZWSP, Treat ZWSP as if it were SP,
        // ZWJ '×', // XXX Handled explicitly // LB8a Do not break after a zero width joiner.
        // Combining marks:
        // LB9 Do not break a combining character sequence; treat it as if it has the line breaking
        // class of the base character in all of the following rules. Treat ZWJ as if it were CM.
        Treat X (CM | ZWJ)* as if it were X where X = [^BK CR LF NL SP ZW sot eot ZWSP OPSP QUSP CLSP CPSP B2SP],
        Treat (CM | ZWJ) as if it were AL, // LB10 Treat any remaining combining mark or ZWJ as AL.
        // Word joiner:
        '×' WJ, WJ '×', // LB11 Do not break before or after Word joiner and related characters.
        // Non-breaking characters:
        GL '×', // LB12 Do not break after NBSP and related characters.

        // Tailorable Line Breaking Rules
        [^SP BA HY sot eot ZWSP OPSP QUSP CLSP CPSP B2SP] '×' GL, // LB12a Do not break before NBSP and related characters, except after spaces and hyphens.
        // LB13 Do not break before ‘]’ or ‘!’ or ‘;’ or ‘/’, even after spaces.
        '×' CL, '×' CP, '×' EX, '×' IS, '×' SY,
        // LB14 Do not break after ‘[’, even after spaces.
        (OP | OPSP) '×', Treat (OP | OPSP) SP as if it were OPSP, Treat ZWSP as if it were SP,
        // LB15 Do not break within ‘”[’, even with intervening spaces.
        (QU | QUSP) '×' OP, Treat (QU | QUSP) SP as if it were QUSP, Treat QUSP as if it were SP,
        // LB16 Do not break between closing punctuation and a nonstarter (lb=NS), even with
        // intervening spaces.
        (CL | CLSP | CP | CPSP) '×' NS,
        Treat (CL | CLSP) SP as if it were CLSP, Treat CLSP as if it were SP,
        Treat (CP | CPSP) SP as if it were CPSP, Treat CPSP as if it were SP,
        // LB17 Do not break within ‘——’, even with intervening spaces.
        (B2 | B2SP) '×' B2, Treat (B2 | B2SP) SP as if it were B2SP, Treat B2SP as if it were SP,
        // Spaces:
        SP '÷', // LB18 Break after spaces.
        // Special case rules:
        '×' QU, QU '×', // LB19 Do not break before or after quotation marks, such as ‘”’.
        '÷' CB, CB '÷', // LB20 Break before and after unresolved CB.
        // LB21 Do not break before hyphen-minus, other hyphens, fixed-width spaces, small kana,
        // and other non-starters, or after acute accents.
        '×' BA, '×' HY, '×' NS, BB '×',
        // LB21a Don't break after Hebrew + Hyphen. // XXX Use a single state, HLHYBA, for HLHY and HLBA
        HLHYBA '×', Treat HL (HY | BA) as if it were HLHYBA, Treat HLHYBA as if it were HY,
        SY '×' HL, // LB21b Don’t break between Solidus and Hebrew letters.
        // LB22 Do not break between two ellipses, or between letters, numbers or exclamations and
        // ellipsis.
        (AL | HL) '×' IN, EX '×' IN, (ID | EB | EM) '×' IN, IN '×' IN, NU '×' IN,
        // Numbers:
        (AL | HL) '×' NU, NU '×' (AL | HL), // LB23 Do not break between digits and letters.
        // LB23a Do not break between numeric prefixes and ideographs, or between ideographs and
        // numeric postfixes.
        PR '×' (ID | EB | EM), (ID | EB | EM) '×' PO,
        // LB24 Do not break between numeric prefix/postfix and letters, or between letters and
        // prefix/postfix.
        (PR | PO) '×' (AL | HL), (AL | HL) '×' (PR | PO),
        // LB25 Do not break between the following pairs of classes relevant to numbers:
        CL '×' PO, CP '×' PO, CL '×' PR, CP '×' PR, NU '×' PO, NU '×' PR, PO '×' OP, PO '×' NU, PR '×' OP, PR '×' NU, HY '×' NU, IS '×' NU, NU '×' NU, SY '×' NU,
        // Korean syllable blocks
        // LB26 Do not break a Korean syllable.
        JL '×' (JL | JV | H2 | H3), (JV | H2) '×' (JV | JT), (JT | H3) '×' JT,
        // LB27 Treat a Korean Syllable Block the same as ID.
        (JL | JV | JT | H2 | H3) '×' IN, (JL | JV | JT | H2 | H3) '×' PO, PR '×' (JL | JV | JT | H2 | H3),
        // Finally, join alphabetic letters into words and break everything else.
        (AL | HL) '×' (AL | HL), // LB28 Do not break between alphabetics (“at”).
        IS '×' (AL | HL), // LB29 Do not break between numeric punctuation and alphabetics (“e.g.”).
        // LB30 Do not break between letters, numbers, or ordinary symbols and opening or closing
        // parentheses.
        (AL | HL | NU) '×' OP, CP '×' (AL | HL | NU),
        // LB30a Break between two regional indicator symbols if and only if there are an even
        // number of regional indicators preceding the position of the break.
        RI '×' RI, Treat RI RI as if it were RIRI, Treat RIRI as if it were RI,
        EB '×' EM, // LB30b Do not break between an emoji base and an emoji modifier.
        '÷' ALL, ALL '÷', // LB31 Break everywhere else.
    };

    // Synthesize all non-"safe" pairs from pair table. There are generally more safe pairs.
    let unsafe_pairs = (0..NUM_CLASSES).into_iter().flat_map(|j| {
        (0..NUM_CLASSES).into_iter().filter_map(move |i| {
            // All states that could have resulted from that break class
            let possible_states = pair_table
                .iter()
                .map(move |row| (row[i] & !(ALLOWED_BREAK_BIT | MANDATORY_BREAK_BIT)) as usize);
            match possible_states
                .map(|i| pair_table[i][j]) // Map to respective state transitions
                // TODO Use Option::xor (see issue #50512)
                // Check if the state transitions are all the same
                .try_fold(None, |acc, x| match acc.filter(|&prev| x != prev) {
                    Some(_) => None,
                    None => Some(Some(x)),
                }) {
                Some(_) => None,
                None => Some((i, j)),
            }
        })
    });

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

            page.clear(); // Reset values to default
        }
    }

    writeln!(
        stream,
        r"];

        const PAGE_COUNT: usize = {};
        const UNIFORM_PAGE: usize = {};
        static PAGE_INDICES: [usize; {}] = [",
        page_count,
        UNIFORM_PAGE,
        page_indices.len()
    )?;
    for page_idx in page_indices {
        write!(stream, "{},", page_idx)?;
    }
    write!(
        stream,
        r"];

        const ALLOWED_BREAK_BIT: u8 = {};
        const MANDATORY_BREAK_BIT: u8 = {};
        const SOT: u8 = {};
        const EOT: u8 = {};
        static PAIR_TABLE: [[u8; {}]; {}] = [",
        ALLOWED_BREAK_BIT,
        MANDATORY_BREAK_BIT,
        state_by_key("sot"),
        state_by_key("eot"),
        NUM_STATES,
        NUM_CLASSES + NUM_EXTRA_STATES
    )?;
    for row in pair_table.iter() {
        write!(stream, "[")?;
        for x in row.iter() {
            write!(stream, "{},", x)?;
        }
        write!(stream, "],")?;
    }
    writeln!(
        stream,
        r"];

        fn is_safe_pair(a: BreakClass, b: BreakClass) -> bool {{
            if let {} = (a, b) {{ false }} else {{ true }}
        }}",
        unsafe_pairs
            .map(|(i, j)| format!("({}, {})", BREAK_CLASS_TABLE[i], BREAK_CLASS_TABLE[j]))
            .collect::<Vec<_>>()
            .join("|")
    )?;

    Ok(())
}
