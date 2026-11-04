// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use unicode_segmentation::UnicodeSegmentation;
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalSendInputMode {
    Text,
    Hex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalSendPacing {
    Line,
    Character,
}

pub enum TerminalSendFrame {
    TextLine(Zeroizing<String>),
    TextChunk(Zeroizing<String>),
    RawBytes(Zeroizing<Vec<u8>>),
}

impl TerminalSendFrame {
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::TextLine(text) | Self::TextChunk(text) => Some(text),
            Self::RawBytes(_) => None,
        }
    }

    pub fn raw_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::RawBytes(bytes) => Some(bytes),
            Self::TextLine(_) | Self::TextChunk(_) => None,
        }
    }
}

impl fmt::Debug for TerminalSendFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, byte_len) = match self {
            Self::TextLine(text) => ("text-line", text.len()),
            Self::TextChunk(text) => ("text-chunk", text.len()),
            Self::RawBytes(bytes) => ("raw-bytes", bytes.len()),
        };
        formatter
            .debug_struct("TerminalSendFrame")
            .field("kind", &kind)
            .field("byte_len", &byte_len)
            .finish()
    }
}

pub struct TerminalSendPlan {
    frames: Vec<TerminalSendFrame>,
    repeat_count: u32,
    total_units: u64,
}

impl TerminalSendPlan {
    pub fn frames(&self) -> &[TerminalSendFrame] {
        &self.frames
    }

    pub fn repeat_count(&self) -> u32 {
        self.repeat_count
    }

    pub fn total_units(&self) -> u64 {
        self.total_units
    }

    pub fn is_infinite(&self) -> bool {
        self.repeat_count == 0
    }
}

impl fmt::Debug for TerminalSendPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TerminalSendPlan")
            .field("frame_count", &self.frames.len())
            .field("repeat_count", &self.repeat_count)
            .field("total_units", &self.total_units)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalSendPlanError {
    EmptyInput,
    EmptyHexInput,
    InvalidHexDigit,
    OddHexDigitCount,
    ZeroRepeatCount,
    UnitCountOverflow,
}

impl fmt::Display for TerminalSendPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyInput => "composer input is empty",
            Self::EmptyHexInput => "hex composer input contains no bytes",
            Self::InvalidHexDigit => "hex composer input contains an invalid digit",
            Self::OddHexDigitCount => "hex composer input contains an odd number of digits",
            Self::ZeroRepeatCount => "composer repeat count must be at least one",
            Self::UnitCountOverflow => "composer unit count exceeds the supported range",
        })
    }
}

impl std::error::Error for TerminalSendPlanError {}

pub fn build_terminal_send_plan(
    input: Zeroizing<String>,
    input_mode: TerminalSendInputMode,
    pacing: TerminalSendPacing,
    repeat_count: u32,
) -> Result<TerminalSendPlan, TerminalSendPlanError> {
    if input.is_empty() {
        return Err(TerminalSendPlanError::EmptyInput);
    }
    // The normalized copy and every derived frame zeroize independently. This
    // keeps async dispatch captures bounded even when commands contain secrets.
    let frames = match input_mode {
        TerminalSendInputMode::Text => build_text_frames(&input, pacing),
        TerminalSendInputMode::Hex => build_hex_frames(&input, pacing)?,
    };
    if frames.is_empty() {
        return Err(match input_mode {
            TerminalSendInputMode::Text => TerminalSendPlanError::EmptyInput,
            TerminalSendInputMode::Hex => TerminalSendPlanError::EmptyHexInput,
        });
    }
    let frame_count =
        u64::try_from(frames.len()).map_err(|_| TerminalSendPlanError::UnitCountOverflow)?;
    let total_units = if repeat_count == 0 {
        0
    } else {
        checked_total_units(frame_count, repeat_count)?
    };

    Ok(TerminalSendPlan {
        frames,
        repeat_count,
        total_units,
    })
}

fn checked_total_units(frame_count: u64, repeat_count: u32) -> Result<u64, TerminalSendPlanError> {
    frame_count
        .checked_mul(u64::from(repeat_count))
        .ok_or(TerminalSendPlanError::UnitCountOverflow)
}

fn build_text_frames(input: &str, pacing: TerminalSendPacing) -> Vec<TerminalSendFrame> {
    let normalized = Zeroizing::new(input.replace("\r\n", "\n").replace('\r', "\n"));
    match pacing {
        TerminalSendPacing::Line => normalized
            .split_terminator('\n')
            .map(|line| TerminalSendFrame::TextLine(Zeroizing::new(line.to_string())))
            .collect(),
        TerminalSendPacing::Character => normalized
            .graphemes(true)
            .map(|grapheme| {
                TerminalSendFrame::TextChunk(Zeroizing::new(if grapheme == "\n" {
                    "\r".to_string()
                } else {
                    grapheme.to_string()
                }))
            })
            .collect(),
    }
}

fn build_hex_frames(
    input: &str,
    pacing: TerminalSendPacing,
) -> Result<Vec<TerminalSendFrame>, TerminalSendPlanError> {
    match pacing {
        TerminalSendPacing::Line => {
            let normalized = Zeroizing::new(input.replace("\r\n", "\n").replace('\r', "\n"));
            let mut frames = Vec::new();
            for line in normalized.split('\n') {
                let bytes = parse_hex_bytes(line)?;
                if !bytes.is_empty() {
                    frames.push(TerminalSendFrame::RawBytes(bytes));
                }
            }
            Ok(frames)
        }
        TerminalSendPacing::Character => {
            let bytes = parse_hex_bytes(input)?;
            Ok(bytes
                .iter()
                .copied()
                .map(|byte| TerminalSendFrame::RawBytes(Zeroizing::new(vec![byte])))
                .collect())
        }
    }
}

fn parse_hex_bytes(input: &str) -> Result<Zeroizing<Vec<u8>>, TerminalSendPlanError> {
    let mut bytes = Zeroizing::new(Vec::new());
    for token in input.split(|character: char| {
        character.is_ascii_whitespace() || matches!(character, ',' | ':' | ';' | '-' | '_')
    }) {
        if token.is_empty() {
            continue;
        }
        let digits = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
            .unwrap_or(token);
        if digits.is_empty() {
            return Err(TerminalSendPlanError::InvalidHexDigit);
        }
        if !digits.is_ascii() || !digits.as_bytes().iter().all(u8::is_ascii_hexdigit) {
            return Err(TerminalSendPlanError::InvalidHexDigit);
        }
        if digits.len() % 2 != 0 {
            return Err(TerminalSendPlanError::OddHexDigitCount);
        }
        for pair_start in (0..digits.len()).step_by(2) {
            let pair = &digits[pair_start..pair_start + 2];
            let byte =
                u8::from_str_radix(pair, 16).map_err(|_| TerminalSendPlanError::InvalidHexDigit)?;
            bytes.push(byte);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(
        input: &str,
        mode: TerminalSendInputMode,
        pacing: TerminalSendPacing,
        repeat_count: u32,
    ) -> Result<TerminalSendPlan, TerminalSendPlanError> {
        build_terminal_send_plan(
            Zeroizing::new(input.to_string()),
            mode,
            pacing,
            repeat_count,
        )
    }

    #[test]
    fn text_line_plan_normalizes_endings_and_preserves_empty_lines() {
        let plan = plan(
            "echo one\r\n\r\necho two\n",
            TerminalSendInputMode::Text,
            TerminalSendPacing::Line,
            2,
        )
        .unwrap();
        let lines = plan
            .frames()
            .iter()
            .map(|frame| frame.text().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(lines, vec!["echo one", "", "echo two"]);
        assert_eq!(plan.total_units(), 6);
    }

    #[test]
    fn text_character_plan_keeps_graphemes_and_normalizes_line_endings() {
        for (input, expected) in [
            ("a\u{301}👨‍👩‍👧‍👦\n", vec!["a\u{301}", "👨‍👩‍👧‍👦", "\r"]),
            ("a\n\nb\n", vec!["a", "\r", "\r", "b", "\r"]),
        ] {
            let plan = plan(
                input,
                TerminalSendInputMode::Text,
                TerminalSendPacing::Character,
                1,
            )
            .unwrap();
            let chunks = plan
                .frames()
                .iter()
                .map(|frame| frame.text().unwrap())
                .collect::<Vec<_>>();

            assert_eq!(chunks, expected, "{input:?}");
        }
    }

    #[test]
    fn hex_plan_preserves_raw_bytes_and_pacing_units() {
        for (input, pacing, repeats, expected, units) in [
            (
                "0x0d, 0A\ndead-beef",
                TerminalSendPacing::Line,
                1,
                vec![&[0x0d, 0x0a][..], &[0xde, 0xad, 0xbe, 0xef][..]],
                2,
            ),
            (
                "00 ff 7f",
                TerminalSendPacing::Character,
                3,
                vec![&[0x00][..], &[0xff][..], &[0x7f][..]],
                9,
            ),
        ] {
            let plan = plan(input, TerminalSendInputMode::Hex, pacing, repeats).unwrap();
            let frames = plan
                .frames()
                .iter()
                .map(|frame| frame.raw_bytes().unwrap())
                .collect::<Vec<_>>();

            assert_eq!(frames, expected, "{pacing:?}");
            assert_eq!(plan.total_units(), units, "{pacing:?}");
        }
    }

    #[test]
    fn hex_plan_rejects_odd_and_invalid_digits_without_echoing_input() {
        assert_eq!(
            plan(
                "abc",
                TerminalSendInputMode::Hex,
                TerminalSendPacing::Line,
                1,
            )
            .unwrap_err(),
            TerminalSendPlanError::OddHexDigitCount
        );
        assert_eq!(
            plan(
                "not-secret",
                TerminalSendInputMode::Hex,
                TerminalSendPacing::Line,
                1,
            )
            .unwrap_err(),
            TerminalSendPlanError::InvalidHexDigit
        );
        assert!(
            !TerminalSendPlanError::InvalidHexDigit
                .to_string()
                .contains("not-secret")
        );
        assert_eq!(
            plan(
                "aa 0x",
                TerminalSendInputMode::Hex,
                TerminalSendPacing::Line,
                1,
            )
            .unwrap_err(),
            TerminalSendPlanError::InvalidHexDigit
        );
        assert_eq!(
            plan(
                " , ; ",
                TerminalSendInputMode::Hex,
                TerminalSendPacing::Line,
                1,
            )
            .unwrap_err(),
            TerminalSendPlanError::EmptyHexInput
        );
    }

    #[test]
    fn plan_supports_infinite_repeats_when_count_is_zero() {
        let plan = plan(
            "echo one",
            TerminalSendInputMode::Text,
            TerminalSendPacing::Line,
            0,
        )
        .unwrap();
        assert!(plan.is_infinite());
        assert_eq!(plan.repeat_count(), 0);
        assert_eq!(plan.total_units(), 0);
        assert_eq!(plan.frames().len(), 1);
        assert_eq!(
            checked_total_units(u64::MAX, 2).unwrap_err(),
            TerminalSendPlanError::UnitCountOverflow
        );
    }

    #[test]
    fn plan_debug_redacts_frame_contents() {
        let plan = plan(
            "token=super-secret",
            TerminalSendInputMode::Text,
            TerminalSendPacing::Line,
            1,
        )
        .unwrap();
        let debug = format!("{plan:?} {:?}", plan.frames()[0]);

        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("frame_count"));
    }
}
