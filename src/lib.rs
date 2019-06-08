use std::mem;

/// Unicode line breaking class.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum BreakClass {
    // Non-tailorable
    /// Cause a line break (after)
    Mandatory,
    /// Cause a line break (after), except between CR and LF
    CarriageReturn,
    /// Cause a line break (after)
    LineFeed,
    /// Prohibit a line break between the character and the preceding character
    CombiningMark,
    /// Cause a line break (after)
    NextLine,
    /// Do not occur in well-formed text
    Surrogate,
    /// Prohibit line breaks before and after
    WordJoiner,
    /// Provide a break opportunity
    ZeroWidthSpace,
    /// Prohibit line breaks before and after
    NonBreakingGlue,
    /// Enable indirect line breaks
    Space,
    /// Prohibit line breaks within joiner sequences
    ZeroWidthJoiner,
    // Break opportunities
    /// Provide a line break opportunity before and after the character
    BeforeAndAfter,
    /// Generally provide a line break opportunity after the character
    After,
    /// Generally provide a line break opportunity before the character
    Before,
    /// Provide a line break opportunity after the character, except in numeric context
    Hyphen,
    /// Provide a line break opportunity contingent on additional information
    Contingent,
    // Characters prohibiting certain breaks
    /// Prohibit line breaks before
    ClosePunctuation,
    /// Prohibit line breaks before
    CloseParenthesis,
    /// Prohibit line breaks before
    Exclamation,
    /// Allow only indirect line breaks between pairs
    Inseparable,
    /// Allow only indirect line breaks before
    NonStarter,
    /// Prohibit line breaks after
    OpenPunctuation,
    /// Act like they are both opening and closing
    Quotation,
    // Numeric context
    /// Prevent breaks after any and before numeric
    InfixSeparator,
    /// Form numeric expressions for line breaking purposes
    Numeric,
    /// Do not break following a numeric expression
    Postfix,
    /// Do not break in front of a numeric expression
    Prefix,
    /// Prevent a break before, and allow a break after
    Symbol,
    // Other characters
    /// Act like AL when the resolved EAW is N; otherwise, act as ID
    Ambiguous,
    /// Are alphabetic characters or symbols that are used with alphabetic characters
    Alphabetic,
    /// Treat as NS or ID for strict or normal breaking.
    ConditionalJapaneseStarter,
    /// Do not break from following Emoji Modifier
    EmojiBase,
    /// Do not break from preceding Emoji Base
    EmojiModifier,
    /// Form Korean syllable blocks
    HangulLvSyllable,
    /// Form Korean syllable blocks
    HangulLvtSyllable,
    /// Do not break around a following hyphen; otherwise act as Alphabetic
    HebrewLetter,
    /// Break before or after, except in some numeric context
    Ideographic,
    /// Form Korean syllable blocks
    HangulLJamo,
    /// Form Korean syllable blocks
    HangulVJamo,
    /// Form Korean syllable blocks
    HangulTJamo,
    /// Keep pairs together. For pairs, break before and after other classes
    RegionalIndicator,
    /// Provide a line break opportunity contingent on additional, language-specific context analysis
    ComplexContext,
    /// Have as yet unknown line breaking behavior or unassigned code positions
    Unknown,
}

#[allow(unused_imports)]
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

/// Returns the line break property of the specified code point.
///
/// ```
/// use unicode_linebreak::{BreakClass, break_class};
/// assert_eq!(break_class(0x2CF3), BreakClass::Alphabetic);
/// ```
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
