//! Tag type definitions for supported e-ink NFC displays
//!
//! This module provides type-safe abstractions for different display types,
//! including their image format requirements and protocol selection.

/// Marker type for BWR (3-color: Black, White, Red) image format
#[derive(Debug, Clone, Copy)]
pub struct Bwr;

/// Marker type for BWRY (4-color: Black, White, Red, Yellow) image format
#[derive(Debug, Clone, Copy)]
pub struct Bwry;

/// Image format enum for runtime selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// 3-color: Black, White, Red
    Bwr,
    /// 4-color: Black, White, Red, Yellow
    Bwry,
}

/// Protocol enum for runtime selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// IsoDep BWRY protocol for 4-color displays
    IsodepBwry,
    /// IsoDep GenB protocol for 3-color displays
    IsodepGenb,
}

/// Tag type combining display info, image format, and protocol
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct TagType {
    /// Human-readable display name
    pub name: &'static str,
    /// Display width in pixels
    pub width: usize,
    /// Display height in pixels
    pub height: usize,
    /// Required image format
    pub image_format: ImageFormat,
    /// NFC protocol to use
    pub protocol: Protocol,
}

impl TagType {
    /// 1.54inch e-Paper Y - BWRY 4-color display
    pub const EPAPER_Y_154: TagType = TagType {
        name: "1.54inch e-Paper Y",
        width: 200,
        height: 200,
        image_format: ImageFormat::Bwry,
        protocol: Protocol::IsodepBwry,
    };

    /// 1.54inch e-Paper B - BWR 3-color display
    pub const EPAPER_B_154: TagType = TagType {
        name: "1.54inch e-Paper B",
        width: 200,
        height: 200,
        image_format: ImageFormat::Bwr,
        protocol: Protocol::IsodepGenb,
    };

    /// All supported tag types
    pub const ALL: &'static [TagType] = &[
        Self::EPAPER_Y_154,
        Self::EPAPER_B_154,
    ];

    /// Get tag type by index
    pub fn get(index: usize) -> Option<&'static TagType> {
        Self::ALL.get(index)
    }
}
