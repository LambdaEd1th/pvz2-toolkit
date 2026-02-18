#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtxFormat {
    Rgba8888,
    Rgba4444,
    Rgb565,
    Rgba5551,
    Rgba4444Block,
    Rgb565Block,
    Rgba5551Block,
    Pvrtc4BppRgba,
    Etc1,
    Pvrtc4BppRgbaA8,
    Etc1A8,
    Etc1Palette,
    Unknown(i32),
}

impl From<i32> for PtxFormat {
    fn from(v: i32) -> Self {
        match v {
            0 => PtxFormat::Rgba8888,
            1 => PtxFormat::Rgba4444,
            2 => PtxFormat::Rgb565,
            3 => PtxFormat::Rgba5551,
            21 => PtxFormat::Rgba4444Block,
            22 => PtxFormat::Rgb565Block,
            23 => PtxFormat::Rgba5551Block,
            30 => PtxFormat::Pvrtc4BppRgba, // Can also be Etc1Palette on iOS
            147 => PtxFormat::Etc1,         // Can also be Etc1A8 on Android
            148 => PtxFormat::Pvrtc4BppRgbaA8,
            n => PtxFormat::Unknown(n),
        }
    }
}
