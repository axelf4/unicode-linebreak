use std::mem;

/** Unicode Line Break property values. */
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum BreakClass {
    Mandatory,
    CarriageReturn,
    LineFeed,
    CombiningMark,
    Surrogate,
    ZeroWidthSpace,
    Inseparable,
    NonBreakingGlue,
    Contingent,
    Space,
    After,
    Before,
    BeforeAndAfter,
    Hyphen,
    NonStarter,
    OpenPunctuation,
    ClosePunctuation,
    Quotation,
    Exclamation,
    Ideographic,
    Numeric,
    InfixSeparator,
    Symbol,
    Alphabetic,
    Prefix,
    Postfix,
    ComplexContext,
    Ambiguous,
    Unknown,
    NextLine,
    WordJoiner,
    HangulLJamo,
    HangulVJamo,
    HangulTJamo,
    HangulLvSyllable,
    HangulLvtSyllable,
    CloseParenthesis,
    ConditionalJapaneseStarter,
    HebrewLetter,
    RegionalIndicator,
    EmojiBase,
    EmojiModifier,
    ZeroWidthJoiner,
}

#[allow(unused)]
use self::BreakClass::{
    After as BA, Alphabetic as AL, Ambiguous as AI, Before as BB, BeforeAndAfter as B2,
    CarriageReturn as CR, CloseParenthesis as CP, ClosePunctuation as CL, CombiningMark as CM,
    ComplexContext as SA, ConditionalJapaneseStarter as CJ, Contingent as CB, EmojiBase as EB,
    EmojiModifier as EM, Exclamation as EX, HangulLJamo as JL, HangulLvSyllable as H2,
    HangulLvtSyllable as H3, HangulTJamo as JT, HangulVJamo as JV, HebrewLetter as HL,
    Hyphen as HY, Ideographic as ID, InfixSeparator as IS, Inseparable as IN, LineFeed as LF,
    Mandatory as BK, NextLine as NL, NonBreakingGlue as GL, NonStarter as NS, Numeric as NU,
    OpenPunctuation as OP, Postfix as PO, Prefix as PR, Quotation as QU, RegionalIndicator as RI,
    Space as SP, Surrogate as SG, Symbol as SY, Unknown as XX, WordJoiner as WJ,
    ZeroWidthJoiner as ZWJ, ZeroWidthSpace as ZW,
};

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

/**
Returns the line break property of the specified code point value.

```rust
use unicode_linebreak::{BreakClass, break_class};
assert_eq!(break_class(0x2CF3), BreakClass::Alphabetic);
```
*/
pub fn break_class(codepoint: u32) -> BreakClass {
    let codepoint = codepoint as usize;
    if (PAGE_INDICES[codepoint >> 8] & UNIFORM_PAGE) != 0 {
        unsafe { mem::transmute((PAGE_INDICES[codepoint >> 8] & !UNIFORM_PAGE) as u8) }
    } else {
        BREAK_PROP_DATA[PAGE_INDICES[codepoint >> 8]][codepoint & 0xFF]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(break_class(0xA), BreakClass::LineFeed);
        assert_eq!(break_class(0xDB80), BreakClass::Surrogate);
    }
}
