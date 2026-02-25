use crate::error::{Result, RsbError};
use crate::ptx::codec::etc1::{encode_etc1_block, encode_palette_alpha};
use crate::ptx::color::Rgba32;
use crate::ptx::types::PtxFormat;
use image::{DynamicImage, GenericImageView};

pub struct PtxEncoder;

impl PtxEncoder {
    pub fn encode(image: &DynamicImage, format: PtxFormat, is_powervr: bool) -> Result<Vec<u8>> {
        let width = image.width();
        let height = image.height();

        match format {
            PtxFormat::Rgba8888 => {
                // 4 bytes per pixel
                let mut data = Vec::with_capacity((width * height * 4) as usize);
                for y in 0..height {
                    for x in 0..width {
                        let p = image.get_pixel(x, y);
                        if is_powervr {
                            data.push(p[2]); // B
                            data.push(p[1]); // G
                            data.push(p[0]); // R
                            data.push(p[3]); // A
                        } else {
                            data.push(p[0]); // R
                            data.push(p[1]); // G
                            data.push(p[2]); // B
                            data.push(p[3]); // A
                        }
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgba4444 => {
                // 2 bytes per pixel
                let mut data = Vec::with_capacity((width * height * 2) as usize);
                for y in 0..height {
                    for x in 0..width {
                        let p = image.get_pixel(x, y);
                        // 8-bit to 4-bit: (c >> 4)
                        let r = (p[0] >> 4) as u16;
                        let g = (p[1] >> 4) as u16;
                        let b = (p[2] >> 4) as u16;
                        let a = (p[3] >> 4) as u16;

                        // RRRR GGGG BBBB AAAA
                        let val = (r << 12) | (g << 8) | (b << 4) | a;
                        data.extend_from_slice(&val.to_le_bytes());
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgb565 => {
                let mut data = Vec::with_capacity((width * height * 2) as usize);
                for y in 0..height {
                    for x in 0..width {
                        let p = image.get_pixel(x, y);
                        // R5 G6 B5
                        // R: 5 bits (high 5 of 8) -> p[0] >> 3
                        // G: 6 bits (high 6 of 8) -> p[1] >> 2
                        // B: 5 bits (high 5 of 8) -> p[2] >> 3

                        let r = (p[0] >> 3) as u16;
                        let g = (p[1] >> 2) as u16;
                        let b = (p[2] >> 3) as u16;

                        // RRRRR GGGGGG BBBBB
                        let val = (r << 11) | (g << 5) | b;
                        data.extend_from_slice(&val.to_le_bytes());
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgba5551 => {
                let mut data = Vec::with_capacity((width * height * 2) as usize);
                for y in 0..height {
                    for x in 0..width {
                        let p = image.get_pixel(x, y);
                        // R5 G5 B5 A1
                        let r = (p[0] >> 3) as u16;
                        let g = (p[1] >> 3) as u16;
                        let b = (p[2] >> 3) as u16;
                        let a = if p[3] > 127 { 1 } else { 0 } as u16;

                        // RRRRR GGGGG BBBBB A
                        let val = (r << 11) | (g << 6) | (b << 1) | a;
                        data.extend_from_slice(&val.to_le_bytes());
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgba4444Block => {
                // Tiled 32x32 blocks
                let block_w = 32;
                let block_h = 32;
                let blocks_x = width.div_ceil(block_w);
                let blocks_y = height.div_ceil(block_h);

                let mut data = Vec::with_capacity((width * height * 2) as usize);

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    let p = image.get_pixel(x, y);
                                    let r = (p[0] >> 4) as u16;
                                    let g = (p[1] >> 4) as u16;
                                    let b = (p[2] >> 4) as u16;
                                    let a = (p[3] >> 4) as u16;
                                    let val = (r << 12) | (g << 8) | (b << 4) | a;
                                    data.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    // Padding
                                    data.extend_from_slice(&[0, 0]);
                                }
                            }
                        }
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgb565Block => {
                let block_w = 32;
                let block_h = 32;
                let blocks_x = width.div_ceil(block_w);
                let blocks_y = height.div_ceil(block_h);

                let mut data = Vec::with_capacity((width * height * 2) as usize);

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    let p = image.get_pixel(x, y);
                                    let r = (p[0] >> 3) as u16;
                                    let g = (p[1] >> 2) as u16;
                                    let b = (p[2] >> 3) as u16;
                                    let val = (r << 11) | (g << 5) | b;
                                    data.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    data.extend_from_slice(&[0, 0]);
                                }
                            }
                        }
                    }
                }
                Ok(data)
            }
            PtxFormat::Rgba5551Block => {
                let block_w = 32;
                let block_h = 32;
                let blocks_x = width.div_ceil(block_w);
                let blocks_y = height.div_ceil(block_h);

                let mut data = Vec::with_capacity((width * height * 2) as usize);

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    let p = image.get_pixel(x, y);
                                    let r = (p[0] >> 3) as u16;
                                    let g = (p[1] >> 3) as u16;
                                    let b = (p[2] >> 3) as u16;
                                    let a = if p[3] > 127 { 1 } else { 0 } as u16;
                                    let val = (r << 11) | (g << 6) | (b << 1) | a;
                                    data.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    data.extend_from_slice(&[0, 0]);
                                }
                            }
                        }
                    }
                }
                Ok(data)
            }
            PtxFormat::Etc1 => {
                // Encode standard ETC1 (Opaque)
                // Need 4x4 blocks
                let mut data = Vec::new();
                let blocks_x = width.div_ceil(4);
                let blocks_y = height.div_ceil(4);

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        // Extract 4x4 block
                        let mut block_pixels = [Rgba32::default(); 16];
                        for y in 0..4 {
                            for x in 0..4 {
                                let px = bx * 4 + x;
                                let py = by * 4 + y;
                                if px < width && py < height {
                                    block_pixels[(y * 4 + x) as usize] =
                                        Rgba32::from_pixel(image.get_pixel(px, py));
                                } else {
                                    // Pad with black or edge?
                                    block_pixels[(y * 4 + x) as usize] = Rgba32::new(0, 0, 0, 255);
                                }
                            }
                        }

                        let encoded_block = encode_etc1_block(&block_pixels);
                        data.extend_from_slice(&encoded_block.to_be_bytes()); // ETC1 is Big Endian
                    }
                }
                Ok(data)
            }
            PtxFormat::Etc1A8 => {
                // Encode as ETC1 + Uncompressed Alpha (Legacy/Standard)
                // Pass 1: RGB (ETC1)
                let mut data = Vec::new();
                let blocks_x = width.div_ceil(4);
                let blocks_y = height.div_ceil(4);

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        let mut block_pixels = [Rgba32::default(); 16];
                        for y in 0..4 {
                            for x in 0..4 {
                                let px = bx * 4 + x;
                                let py = by * 4 + y;
                                if px < width && py < height {
                                    block_pixels[(y * 4 + x) as usize] =
                                        Rgba32::from_pixel(image.get_pixel(px, py));
                                } else {
                                    block_pixels[(y * 4 + x) as usize] = Rgba32::new(0, 0, 0, 255);
                                }
                            }
                        }
                        let encoded_block = encode_etc1_block(&block_pixels);
                        data.extend_from_slice(&encoded_block.to_be_bytes());
                    }
                }

                // Pass 2: Alpha (Uncompressed)
                for y in 0..height {
                    for x in 0..width {
                        data.push(image.get_pixel(x, y)[3]);
                    }
                }
                Ok(data)
            }
            PtxFormat::Etc1Palette => {
                // Encode using Palette Alpha logic
                encode_palette_alpha(image)
            }
            _ => Err(RsbError::DeserializationError(format!(
                "Encoding not implemented for {:?}",
                format
            ))),
        }
    }
}
