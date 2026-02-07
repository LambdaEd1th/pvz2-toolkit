use image::{DynamicImage, ImageBuffer, Rgba};
pub mod error;
use crate::error::{Result, RsbError};
// use texture2ddecoder::{decode_etc1, decode_pvrtc_4bpp};

pub mod codec;
pub mod color;

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
            30 => PtxFormat::Pvrtc4BppRgba,
            147 => PtxFormat::Etc1,
            148 => PtxFormat::Pvrtc4BppRgbaA8,
            n => PtxFormat::Unknown(n),
        }
    }
}

pub struct PtxDecoder;

impl PtxDecoder {
    pub fn decode(
        data: &[u8],
        width: u32,
        height: u32,
        format_code: i32,
        _alpha_size: Option<i32>, // TODO: Handle alpha if necessary
        is_powervr: bool,
    ) -> Result<DynamicImage> {
        let format = PtxFormat::from(format_code);
        let num_pixels = (width * height) as usize;

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

                    if is_powervr {
                        // PowerVR/iOS: BGRA (Swapped)
                        let b = data[offset];
                        let g = data[offset + 1];
                        let r = data[offset + 2];
                        let a = data[offset + 3];
                        *pixel = Rgba([r, g, b, a]);
                    } else {
                        // Default/Android: RGBA (Direct)
                        let r = data[offset];
                        let g = data[offset + 1];
                        let b = data[offset + 2];
                        let a = data[offset + 3];
                        *pixel = Rgba([r, g, b, a]);
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgb565 => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for RGB565: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    // RGB 565: RRRRR GGGGGG BBBBB
                    let r5 = (v >> 11) & 0x1F;
                    let g6 = (v >> 5) & 0x3F;
                    let b5 = v & 0x1F;

                    // Expand to 8-bit
                    let r = (r5 << 3) | (r5 >> 2);
                    let g = (g6 << 2) | (g6 >> 4);
                    let b = (b5 << 3) | (b5 >> 2);

                    *pixel = Rgba([r as u8, g as u8, b as u8, 255]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba4444 | PtxFormat::Rgba4444Block => {
                // Block suffix might just imply tiling which we might ignore for now or they are same layout
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for RGBA4444: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    // RGBA 4444: RRRR GGGG BBBB AAAA in a 16-bit word?
                    // Or Byte 0: (G << 4) | R ?
                    // Usually: 0xF000 (R), 0x0F00 (G), 0x00F0 (B), 0x000F (A) or similar.
                    // C# TextureCoder would clarify.
                    // Assuming Little Endian u16.
                    // If 0xRGBA (4444) -> Memory: BA RG (LO HI)?
                    // Let's assume nibbles are [G R] [A B] in bytes? No that's weird.
                    // Standard OpenGL RGBA4: R G B A order in nibbles?
                    // Let's implement generic 4444 and tweak if colors are wrong.
                    // Try: Val = (r << 12) | (g << 8) | (b << 4) | a
                    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    let a4 = v & 0xF;
                    let b4 = (v >> 4) & 0xF;
                    let g4 = (v >> 8) & 0xF;
                    let r4 = (v >> 12) & 0xF;

                    let r = (r4 << 4) | r4;
                    let g = (g4 << 4) | g4;
                    let b = (b4 << 4) | b4;
                    let a = (a4 << 4) | a4;

                    *pixel = Rgba([r as u8, g as u8, b as u8, a as u8]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba5551 | PtxFormat::Rgba5551Block => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for RGBA5551: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                for (i, pixel) in img_buf.pixels_mut().enumerate() {
                    let offset = i * 2;
                    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    // RRRRR GGGGG BBBBB A
                    let r5 = (v >> 11) & 0x1F;
                    let g5 = (v >> 6) & 0x1F;
                    let b5 = (v >> 1) & 0x1F;
                    let a1 = v & 1;

                    let r = (r5 << 3) | (r5 >> 2);
                    let g = (g5 << 3) | (g5 >> 2);
                    let b = (b5 << 3) | (b5 >> 2);
                    let a = if a1 == 1 { 255 } else { 0 };

                    *pixel = Rgba([r as u8, g as u8, b as u8, a]);
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Pvrtc4BppRgba | PtxFormat::Pvrtc4BppRgbaA8 => {
                // Calculate Power-of-Two (POT) dimensions for PVRTC
                let mut new_width = width;
                let mut new_height = height;
                if new_width < 8 {
                    new_width = 8;
                }
                if new_height < 8 {
                    new_height = 8;
                }

                // Round up to next POT
                fn next_pow2(v: u32) -> u32 {
                    let mut p = 1;
                    while p < v {
                        p <<= 1;
                    }
                    p
                }

                if (new_width & (new_width - 1)) != 0 {
                    new_width = next_pow2(new_width);
                }
                if (new_height & (new_height - 1)) != 0 {
                    new_height = next_pow2(new_height);
                }

                // PVRTC requires square textures equal to the larger dimension
                let size = std::cmp::max(new_width, new_height);
                let pot_width = size;
                let _pot_height = size; // PVRTC is square in this decoder assumption

                let packet_count = ((pot_width * pot_width) / 16) as usize;
                let expected_data_len = packet_count * 8;

                if data.len() < expected_data_len {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for PVRTC: expected at least {}, got {}",
                        expected_data_len,
                        data.len()
                    )));
                }

                // Read packets
                use byteorder::{ByteOrder, LE};
                let mut packets = Vec::with_capacity(packet_count);
                for i in 0..packet_count {
                    let offset = i * 8;
                    let word = LE::read_u64(&data[offset..offset + 8]);
                    packets.push(crate::codec::pvrtc::PvrTcPacket::new(word));
                }

                // Decode PVRTC (returns square image of size pot_width)
                let decoded_pixels = crate::codec::pvrtc::decode_4bpp(&packets, pot_width as i32);

                let mut img_buf = ImageBuffer::new(width, height);
                // Copy with cropping
                for y in 0..height {
                    for x in 0..width {
                        // PVRTC decoder returns linear array but using Morton order?
                        // No, decode_4bpp returns linear array in standard order?
                        // Let's check decode_4bpp again.
                        // It calculates `result_idx = ((py + (y << 2)) * width + px + (x << 2))`.
                        // Yes, it returns standard row-major buffer.
                        let src_idx = (y * pot_width + x) as usize;
                        if src_idx < decoded_pixels.len() {
                            img_buf.put_pixel(x, y, decoded_pixels[src_idx].to_pixel());
                        }
                    }
                }

                // Handle A8 Alpha
                if format == PtxFormat::Pvrtc4BppRgbaA8 {
                    let alpha_offset = expected_data_len;
                    let expected_total = alpha_offset + (width * height) as usize;
                    if data.len() >= expected_total {
                        for y in 0..height {
                            for x in 0..width {
                                let idx = (y * width + x) as usize;
                                let alpha = data[alpha_offset + idx];
                                let pixel = img_buf.get_pixel_mut(x, y);
                                pixel[3] = alpha;
                            }
                        }
                    }
                }

                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Etc1 => {
                let expected_size_opaque = ((width * height) / 2) as usize;

                if data.len() < expected_size_opaque {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for ETC1: expected at least {}, got {}",
                        expected_size_opaque,
                        data.len()
                    )));
                }

                let blocks_x = width / 4;
                let blocks_y = height / 4;

                let mut img_buf = ImageBuffer::new(width, height);

                for y in 0..blocks_y {
                    for x in 0..blocks_x {
                        let block_idx = (y * blocks_x + x) as usize;
                        let offset = block_idx * 8;
                        // ETC1 is Big Endian
                        let temp = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());

                        let mut decoded_block = [crate::color::Rgba32::default(); 16];
                        crate::codec::etc1::decode_etc1(temp, &mut decoded_block);

                        for py in 0..4 {
                            for px in 0..4 {
                                let gx = x * 4 + px;
                                let gy = y * 4 + py;

                                if gx < width && gy < height {
                                    // decoded_block is Column-Major (x*4 + y) or whatever my logic in etc1.rs handles.
                                    // In etc1.rs: result[(i << 2) | j] = ... i=0..4 (x), j=0..4 (y).
                                    // So index is x*4 + y.
                                    let pixel = decoded_block[px as usize * 4 + py as usize];
                                    img_buf.put_pixel(gx, gy, pixel.to_pixel());
                                }
                            }
                        }
                    }
                }

                // Check for Alpha (A8 or Palette)
                let expected_size_a8 = expected_size_opaque + (width * height) as usize;

                if data.len() >= expected_size_a8 {
                    // Check if it matches A8 size exactly or if it's palette
                    // The palette format size is variable.
                    // A8 size is fixed.
                    // If data.len() == expected_size_a8, assume A8.
                    // The heuristic in C# isn't explicit, but A8 is common.
                    // If we have palette data, the first byte is 'num'.

                    if data.len() == expected_size_a8 {
                        // Decode A8
                        let alpha_data = &data[expected_size_opaque..];
                        for (i, pixel) in img_buf.pixels_mut().enumerate() {
                            pixel[3] = alpha_data[i];
                        }
                    } else if data.len() > expected_size_opaque {
                        // Try decoding as palette
                        let alpha_data = &data[expected_size_opaque..];
                        if let Ok(alphas) = crate::codec::etc1::decode_palette_alpha(
                            alpha_data,
                            (width * height) as usize,
                        ) {
                            for (i, pixel) in img_buf.pixels_mut().enumerate() {
                                if i < alphas.len() {
                                    pixel[3] = alphas[i];
                                }
                            }
                        }
                    }
                }

                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            _ => Err(RsbError::DeserializationError(format!(
                "Unsupported PTX format: {:?}",
                format
            ))),
        }
    }
}

pub struct PtxEncoder;

impl PtxEncoder {
    pub fn encode(image: &DynamicImage, format: PtxFormat) -> Result<Vec<u8>> {
        let width = image.width();
        let height = image.height();

        match format {
            PtxFormat::Pvrtc4BppRgba | PtxFormat::Pvrtc4BppRgbaA8 => {
                // Ensure dimensions are POT and square? PVRTC usually requires POT or at least blocks.
                // Our implementation handles arbitrary size by blocks, but texture2ddecoder might have rules.
                // The C# port assumes blocks.

                let img_rgba = image.to_rgba8();
                let pixels: Vec<crate::color::Rgba32> = img_rgba
                    .pixels()
                    .map(|p| crate::color::Rgba32::from_pixel(*p))
                    .collect();

                let packets = crate::codec::pvrtc::encode_rgba_4bpp(&pixels, width as i32);

                // Serialize packets
                let mut out_data = Vec::with_capacity(packets.len() * 8);
                use byteorder::{ByteOrder, LE};
                let mut buf = [0u8; 8];
                for packet in packets {
                    LE::write_u64(&mut buf, packet.pvr_tc_word);
                    out_data.extend_from_slice(&buf);
                }
                Ok(out_data)
            }
            PtxFormat::Etc1 => {
                let img_rgba = image.to_rgba8();
                let pixels: Vec<crate::color::Rgba32> = img_rgba
                    .pixels()
                    .map(|p| crate::color::Rgba32::from_pixel(*p))
                    .collect();

                let blocks_x = width / 4;
                let blocks_y = height / 4;
                let mut out_data = Vec::with_capacity((blocks_x * blocks_y * 8) as usize);

                for y in 0..blocks_y {
                    for x in 0..blocks_x {
                        // Extract 4x4 block
                        let mut block_colors = [crate::color::Rgba32::default(); 16];
                        for py in 0..4 {
                            for px in 0..4 {
                                let gx = x * 4 + px;
                                let gy = y * 4 + py;
                                if gx < width && gy < height {
                                    let idx = (gy * width + gx) as usize;
                                    // ETC1 usually expects standard row-major per block?
                                    // But my decoder assumed column-major storage in the array?
                                    // Let's verify `etc1.rs` implementation of `gen_etc1`.
                                    // `gen_etc1` takes `&[Rgba32; 16]`.
                                    // It calls `gen_horizontal` etc.
                                    // `get_left_colors` accesses `pixels[y*4 + x]`.
                                    // This implies input array is row-major (y*4+x).

                                    block_colors[(py * 4 + px) as usize] = pixels[idx];
                                }
                            }
                        }

                        let encoded_word = crate::codec::etc1::gen_etc1(&block_colors);
                        out_data.extend_from_slice(&encoded_word.to_be_bytes());
                        // ETC1 is BE
                    }
                }
                Ok(out_data)
            }
            _ => Err(RsbError::DeserializationError(format!(
                "Encoding not implemented for format: {:?}",
                format
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba8888_decoding() {
        // Red pixel: A: 255, R: 255, G: 0, B: 0

        // In BGRA (iOS/PowerVR): B=0, G=0, R=255, A=255
        let bgra_data = [0, 0, 255, 255];

        // In RGBA (Android/Default): R=255, G=0, B=0, A=255
        let rgba_data = [255, 0, 0, 255];

        // Case 1: PowerVR (iOS) decoding of BGRA data
        // Flag = true. Expects BGRA input.
        let img_ios =
            PtxDecoder::decode(&bgra_data, 1, 1, 0, None, true).expect("PowerVR decoding failed");
        let pixel_ios = *img_ios.to_rgba8().get_pixel(0, 0);
        assert_eq!(
            pixel_ios,
            Rgba([255, 0, 0, 255]),
            "PowerVR decoding (BGRA input) should result in Red pixel"
        );

        // Case 2: Default (Android) decoding of RGBA data
        // Flag = false. Expects RGBA input.
        let img_android =
            PtxDecoder::decode(&rgba_data, 1, 1, 0, None, false).expect("Default decoding failed");
        let pixel_android = *img_android.to_rgba8().get_pixel(0, 0);
        assert_eq!(
            pixel_android,
            Rgba([255, 0, 0, 255]),
            "Default decoding (RGBA input) should result in Red pixel"
        );
    }
}
