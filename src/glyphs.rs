//! Character sets: the built-in denylist and the matcher palette.

use std::ops::RangeInclusive;

use anyhow::{anyhow, bail};

pub const BUILTIN_DENY: &[RangeInclusive<u32>] = &[
    0x2580..=0x259F,
    0xE000..=0xF8FF,
    0xF0000..=0xFFFFD,
    0x100000..=0x10FFFD,
];

#[derive(Debug, Clone)]
pub struct DenyList {
    ranges: Vec<RangeInclusive<u32>>,
}

impl DenyList {
    /// Builds a denylist from the built-in ranges plus `extra`.
    pub fn new(extra: &[RangeInclusive<u32>]) -> Self {
        let ranges = BUILTIN_DENY.iter().chain(extra).cloned().collect();
        Self { ranges }
    }

    pub fn is_denied(&self, c: char) -> bool {
        let c = c as u32;
        self.ranges.iter().any(|r| r.contains(&c))
    }
}

fn parse_code_point(s: &str) -> Option<u32> {
    let hex = s.strip_prefix("U+").or_else(|| s.strip_prefix("u+"))?;
    u32::from_str_radix(hex, 16).ok()
}

/// Parses `U+XXXX` or `U+XXXX-U+YYYY` into an inclusive range.
pub fn parse_range(s: &str) -> anyhow::Result<RangeInclusive<u32>> {
    let s = s.trim();
    let (a, b) = s.split_once('-').unwrap_or((s, s));
    match (parse_code_point(a.trim()), parse_code_point(b.trim())) {
        (Some(lo), Some(hi)) if lo <= hi => Ok(lo..=hi),
        _ => Err(anyhow!(
            "invalid code point range '{s}' (expected U+XXXX or U+XXXX-U+YYYY)"
        )),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    chars: Vec<char>,
}

impl Palette {
    pub fn ascii95() -> Self {
        Self {
            chars: (' '..='~').collect(),
        }
    }

    /// Builds a sorted, de-duplicated palette from `"ascii95"` or a literal string of characters.
    pub fn parse(spec: &str) -> anyhow::Result<Self> {
        if spec == "ascii95" {
            return Ok(Self::ascii95());
        }
        let mut chars: Vec<char> = spec.chars().collect();
        chars.sort_unstable();
        chars.dedup();
        if chars.is_empty() {
            bail!("palette is empty");
        }
        Ok(Self { chars })
    }

    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Fails with a list of any palette characters that are denied.
    pub fn validate(&self, deny: &DenyList) -> anyhow::Result<()> {
        let denied: Vec<char> = self
            .chars
            .iter()
            .copied()
            .filter(|&c| deny.is_denied(c))
            .collect();
        if !denied.is_empty() {
            bail!("palette contains denied characters: {denied:?}");
        }
        Ok(())
    }
}

// These tests were written by Claude Code to ensure code coverage.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii95_has_95_chars() {
        let p = Palette::ascii95();
        assert_eq!(p.chars().len(), 95);
        assert_eq!(p.chars()[0], ' ');
        assert_eq!(*p.chars().last().unwrap(), '~');
    }

    #[test]
    fn builtin_deny_blocks_and_pua() {
        let d = DenyList::new(&[]);
        for c in [
            '█',
            '▓',
            '▒',
            '░',
            '▀',
            '▄',
            '▌',
            '▐',
            '\u{E000}',
            '\u{F8FF}',
            '\u{F0000}',
        ] {
            assert!(d.is_denied(c), "{c:?}");
        }
        for c in ['a', '#', '@', '─', '⠿', 'λ'] {
            assert!(!d.is_denied(c), "{c:?}");
        }
    }

    #[test]
    fn parse_range_forms() {
        assert_eq!(parse_range("U+2588").unwrap(), 0x2588..=0x2588);
        assert_eq!(parse_range("u+2580-U+259F").unwrap(), 0x2580..=0x259F);
        assert!(parse_range("2588").is_err());
        assert!(parse_range("U+259F-U+2580").is_err());
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn extra_deny_applies() {
        let d = DenyList::new(&[0x40..=0x40]);
        assert!(d.is_denied('@'));
    }

    #[test]
    fn palette_with_denied_char_fails_validation() {
        let p = Palette::parse("ab█").unwrap();
        let err = p.validate(&DenyList::new(&[])).unwrap_err().to_string();
        assert!(err.contains('█'), "{err}");
    }

    #[test]
    fn palette_parse_dedups_and_sorts() {
        assert_eq!(Palette::parse("cabba").unwrap().chars(), &['a', 'b', 'c']);
        assert!(Palette::parse("").is_err());
    }
}
