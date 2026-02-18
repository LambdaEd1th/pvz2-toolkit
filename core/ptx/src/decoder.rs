use crate::codec::etc1::{decode_etc1, decode_etc1_a8, decode_palette_alpha};
use crate::codec::pvrtc::{decode_pvrtc_4bpp, decode_pvrtc_4bpp_a8};
use crate::error::{Result, RsbError};
use crate::types::PtxFormat;
use image::{DynamicImage, ImageBuffer, Rgba};

pub struct PtxDecoder;

impl PtxDecoder {
    pub fn decode(
        data: &[u8],
        width: u32,
        height: u32,
        format_code: i32,
        _alpha_size: Option<i32>,
        _alpha_format: Option<i32>,
        is_powervr: bool,
    ) -> Result<DynamicImage> {
        let mut format = PtxFormat::from(format_code);
        let num_pixels = (width * height) as usize;

        // Resolve ambiguous formats based on data size
        if format_code == 30 {
            // ID 30: PVRTC 4bpp (Opaque) OR ETC1 Palette
            let expected_msg_size = (width * height) as usize / 2;
            if data.len() > expected_msg_size + 16 {
                // significantly larger
                format = PtxFormat::Etc1Palette;
            }
        } else if format_code == 147 {
            // ID 147: ETC1 (Opaque) OR ETC1 A8
            let expected_opaque_size = (width * height) as usize / 2;
            // Check for ETC1 + Uncompressed Alpha (1 byte/pixel)
            if data.len() >= expected_opaque_size + (width * height) as usize {
                format = PtxFormat::Etc1A8;
            } else if data.len() >= expected_opaque_size * 2 {
                // Check for ETC1 + Compressed Alpha (ETC1 block)
                // Use Etc1A8 internally but handle splitting in decoder
                format = PtxFormat::Etc1A8;
            }
        }

        match format {
            PtxFormat::Rgba8888 => {
                if data.len() < num_pixels * 4 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgba8888: expected {}, got {}",
                        num_pixels * 4,
                        data.len()
                    )));
                }

                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 4;
                    let r = data[offset];
                    let g = data[offset + 1];
                    let b = data[offset + 2];
                    let a = data[offset + 3];

                    if is_powervr {
                        // BGRA
                        *pixel = Rgba([b, g, r, a]);
                    } else {
                        *pixel = Rgba([r, g, b, a]);
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba4444 => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgba4444: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    let val = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    // Sen: (val & 0xF000) >> 12 -> R
                    let r = ((val & 0xF000) >> 12) as u8;
                    let g = ((val & 0x0F00) >> 8) as u8;
                    let b = ((val & 0x00F0) >> 4) as u8;
                    let a = (val & 0x000F) as u8;

                    let r8 = (r << 4) | r;
                    let g8 = (g << 4) | g;
                    let b8 = (b << 4) | b;
                    let a8 = (a << 4) | a;

                    *pixel = Rgba([r8, g8, b8, a8]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgb565 => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgb565: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    let val = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    let r = ((val & 0xF800) >> 11) as u8;
                    let g = ((val & 0x07E0) >> 5) as u8;
                    let b = (val & 0x001F) as u8;

                    let r8 = (r << 3) | (r >> 2);
                    let g8 = (g << 2) | (g >> 4);
                    let b8 = (b << 3) | (b >> 2);

                    *pixel = Rgba([r8, g8, b8, 255]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba5551 => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgba5551: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    let val = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    let r = ((val & 0xF800) >> 11) as u8;
                    let g = ((val & 0x07C0) >> 6) as u8;
                    let b = ((val & 0x003E) >> 1) as u8;
                    let a = (val & 0x0001) as u8;

                    let r8 = (r << 3) | (r >> 2);
                    let g8 = (g << 3) | (g >> 2);
                    let b8 = (b << 3) | (b >> 2);
                    let a8 = if a == 1 { 255 } else { 0 };

                    *pixel = Rgba([r8, g8, b8, a8]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba4444Block => {
                let block_w = 32;
                let block_h = 32;

                let mut img_buf = ImageBuffer::new(width, height);
                let blocks_x = (width + block_w - 1) / block_w;
                let blocks_y = (height + block_h - 1) / block_h;

                let mut offset = 0;

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    if offset + 2 <= data.len() {
                                        let val =
                                            u16::from_le_bytes([data[offset], data[offset + 1]]);
                                        let r = ((val & 0xF000) >> 12) as u8;
                                        let g = ((val & 0x0F00) >> 8) as u8;
                                        let b = ((val & 0x00F0) >> 4) as u8;
                                        let a = (val & 0x000F) as u8;

                                        let r8 = (r << 4) | r;
                                        let g8 = (g << 4) | g;
                                        let b8 = (b << 4) | b;
                                        let a8 = (a << 4) | a;

                                        img_buf.put_pixel(x, y, Rgba([r8, g8, b8, a8]));
                                    }
                                }
                                offset += 2;
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgb565Block => {
                let block_w = 32;
                let block_h = 32;

                let mut img_buf = ImageBuffer::new(width, height);
                let blocks_x = (width + block_w - 1) / block_w;
                let blocks_y = (height + block_h - 1) / block_h;

                let mut offset = 0;

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    if offset + 2 <= data.len() {
                                        let val =
                                            u16::from_le_bytes([data[offset], data[offset + 1]]);
                                        let r = ((val & 0xF800) >> 11) as u8;
                                        let g = ((val & 0x07E0) >> 5) as u8;
                                        let b = (val & 0x001F) as u8;

                                        let r8 = (r << 3) | (r >> 2);
                                        let g8 = (g << 2) | (g >> 4);
                                        let b8 = (b << 3) | (b >> 2);

                                        img_buf.put_pixel(x, y, Rgba([r8, g8, b8, 255]));
                                    }
                                }
                                offset += 2;
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba5551Block => {
                let block_w = 32;
                let block_h = 32;

                let mut img_buf = ImageBuffer::new(width, height);
                let blocks_x = (width + block_w - 1) / block_w;
                let blocks_y = (height + block_h - 1) / block_h;

                let mut offset = 0;

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        for y_local in 0..block_h {
                            for x_local in 0..block_w {
                                let x = bx * block_w + x_local;
                                let y = by * block_h + y_local;

                                if x < width && y < height {
                                    if offset + 2 <= data.len() {
                                        let val =
                                            u16::from_le_bytes([data[offset], data[offset + 1]]);
                                        let r = ((val & 0xF800) >> 11) as u8;
                                        let g = ((val & 0x07C0) >> 6) as u8;
                                        let b = ((val & 0x003E) >> 1) as u8;
                                        let a = (val & 0x0001) as u8;

                                        let r8 = (r << 3) | (r >> 2);
                                        let g8 = (g << 3) | (g >> 2);
                                        let b8 = (b << 3) | (b >> 2);
                                        let a8 = if a == 1 { 255 } else { 0 };

                                        img_buf.put_pixel(x, y, Rgba([r8, g8, b8, a8]));
                                    }
                                }
                                offset += 2;
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Pvrtc4BppRgba => decode_pvrtc_4bpp(data, width, height),
            PtxFormat::Etc1 => decode_etc1(data, width, height),
            PtxFormat::Pvrtc4BppRgbaA8 => {
                let alpha_size = (width * height) as usize;
                if data.len() < alpha_size {
                    return Err(RsbError::DeserializationError(
                        "Data too small for PVRTC+A8".into(),
                    ));
                }
                let offset = data.len().saturating_sub(alpha_size);
                let pvrtc_data = &data[..offset];
                let alpha_data = &data[offset..];

                decode_pvrtc_4bpp_a8(pvrtc_data, alpha_data, width, height)
            }
            PtxFormat::Etc1A8 => {
                let opaque_size = (width * height) as usize / 2;
                // Check for Uncompressed Alpha (3x size) first
                if data.len() >= opaque_size * 3 {
                    // Uncompressed
                    let offset = data.len().saturating_sub((width * height) as usize);
                    let etc1_data = &data[..offset];
                    let alpha_data = &data[offset..];
                    decode_etc1_a8(etc1_data, alpha_data, width, height, false)
                } else if data.len() >= opaque_size * 2 {
                    // Likely dual-ETC1 (RGB + Alpha encoded as ETC1)
                    let midpoint = data.len() / 2;
                    let color_data = &data[..midpoint];
                    let alpha_data = &data[midpoint..];
                    decode_etc1_a8(color_data, alpha_data, width, height, true)
                } else {
                    let offset = data.len().saturating_sub((width * height) as usize);
                    let etc1_data = &data[..offset];
                    let alpha_data = &data[offset..];
                    decode_etc1_a8(etc1_data, alpha_data, width, height, false)
                }
            }
            PtxFormat::Etc1Palette => decode_palette_alpha(data, width, height),
            PtxFormat::Unknown(n) => Err(RsbError::DeserializationError(format!(
                "Unknown PTX format: {}",
                n
            ))),
        }
    }
}
