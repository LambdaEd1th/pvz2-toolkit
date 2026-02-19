use crate::decoder::PtxDecoder;
use crate::encoder::PtxEncoder;
use crate::error::Result;
use crate::types::PtxFormat;
use image::DynamicImage;

pub fn decode_ptx(
    data: &[u8],
    width: u32,
    height: u32,
    format_id: u32,
    alpha_format: Option<u32>,
    prev_alpha_format: Option<u32>,
    is_powervr: bool,
) -> Result<DynamicImage> {
    PtxDecoder::decode(
        data,
        width,
        height,
        format_id as i32,
        alpha_format.map(|x| x as i32),
        prev_alpha_format.map(|x| x as i32),
        is_powervr,
    )
}

pub fn encode_ptx(img: &DynamicImage, format: PtxFormat) -> Result<Vec<u8>> {
    PtxEncoder::encode(img, format)
}
