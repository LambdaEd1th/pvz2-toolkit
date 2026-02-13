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

pub struct PtxDecoder;

impl PtxDecoder {
    pub fn decode(
        data: &[u8],
        width: u32,
        height: u32,
        format_code: i32,
        _alpha_size: Option<i32>,
        alpha_format: Option<i32>,
        is_powervr: bool,
    ) -> Result<DynamicImage> {
        let mut format = PtxFormat::from(format_code);
        let num_pixels = (width * height) as usize;

        // Resolve ambiguous formats based on data size
        if format_code == 30 {
            // ID 30: PVRTC 4bpp (Opaque) OR ETC1 Palette
            // PVRTC size: (max(w, h)^2) / 2 (approx for square/pot, but strict calc needed)
            // ETC1 Palette size: (w*h)/2 + 1 + 16 + ...
            // Heuristic: If size is significantly larger than (w*h)/2, try Palette.
            // Actually, PVRTC 4bpp is EXACTLY (w*h)/2 bits = (w*h)/2 bytes? NO. 4bpp = 0.5 bytes/pixel.
            // ETC1 is also 0.5 bytes/pixel.
            // So base size is same.
            // ETC1 Palette has extra alpha data.
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
            PtxFormat::Rgba4444 => {
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
            PtxFormat::Rgba4444Block => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgba4444Block: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                let mut offset = 0;
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;
                                if py < height && px < width {
                                    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                                    let a4 = v & 0xF;
                                    let b4 = (v >> 4) & 0xF;
                                    let g4 = (v >> 8) & 0xF;
                                    let r4 = (v >> 12) & 0xF;

                                    let r = (r4 << 4) | r4;
                                    let g = (g4 << 4) | g4;
                                    let b = (b4 << 4) | b4;
                                    let a = (a4 << 4) | a4;

                                    img_buf.put_pixel(
                                        px,
                                        py,
                                        Rgba([r as u8, g as u8, b as u8, a as u8]),
                                    );
                                }
                                offset += 2;
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgba5551 => {
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
            PtxFormat::Rgba5551Block => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgba5551Block: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }
                let mut img_buf = ImageBuffer::new(width, height);
                let mut offset = 0;
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;
                                if py < height && px < width {
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

                                    img_buf.put_pixel(px, py, Rgba([r as u8, g as u8, b as u8, a]));
                                }
                                offset += 2;
                            }
                        }
                    }
                }
                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            PtxFormat::Rgb565Block => {
                if data.len() < num_pixels * 2 {
                    return Err(RsbError::DeserializationError(format!(
                        "Insufficient data for Rgb565Block: expected {}, got {}",
                        num_pixels * 2,
                        data.len()
                    )));
                }

                let mut img_buf = ImageBuffer::new(width, height);
                // Tiled decoding: 32x32 blocks
                // Logic based on other block formats (assumed) or standard PowerVR tiling?
                // Sen's implementation or PtxEncoder::encode uses 32x32 blocks.
                // Iterate over blocks in the input data order.

                let mut offset = 0;
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;

                                if py < height && px < width {
                                    let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                                    // RGB 565: RRRRR GGGGGG BBBBB
                                    let r5 = (v >> 11) & 0x1F;
                                    let g6 = (v >> 5) & 0x3F;
                                    let b5 = v & 0x1F;

                                    // Expand to 8-bit
                                    let r = (r5 << 3) | (r5 >> 2);
                                    let g = (g6 << 2) | (g6 >> 4);
                                    let b = (b5 << 3) | (b5 >> 2);

                                    img_buf.put_pixel(
                                        px,
                                        py,
                                        Rgba([r as u8, g as u8, b as u8, 255]),
                                    );
                                }
                                offset += 2;
                            }
                        }
                    }
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
                    // It might be a cropped PVRTC or non-square?
                    // But assume standard PVRTC logic.
                    // If size mismatch, maybe it really WAS Etc1Palette but heuristic failed?
                    // But Logic above checks for larger size.
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
            PtxFormat::Etc1 | PtxFormat::Etc1A8 | PtxFormat::Etc1Palette => {
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

                // Helper to decode an ETC1 block to an image buffer at (x*4, y*4)
                let decode_into = |data_slice: &[u8],
                                   target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
                                   write_alpha_to_red: bool| {
                    for y in 0..blocks_y {
                        for x in 0..blocks_x {
                            let block_idx = (y * blocks_x + x) as usize;
                            let offset = block_idx * 8;
                            // ETC1 is Big Endian
                            let temp = u64::from_be_bytes(
                                data_slice[offset..offset + 8].try_into().unwrap(),
                            );

                            let mut decoded_block = [crate::color::Rgba32::default(); 16];
                            crate::codec::etc1::decode_etc1(temp, &mut decoded_block);

                            for py in 0..4 {
                                for px in 0..4 {
                                    let gx = x * 4 + px;
                                    let gy = y * 4 + py;

                                    if gx < width && gy < height {
                                        let pixel = decoded_block[px as usize * 4 + py as usize];
                                        if write_alpha_to_red {
                                            // For alpha texture, we just want the grayscale value.
                                            // ETC1 R=G=B usually for grayscale. Take R.
                                            let p = target.get_pixel_mut(gx, gy);
                                            p[3] = pixel.r;
                                        } else {
                                            target.put_pixel(gx, gy, pixel.to_pixel());
                                        }
                                    }
                                }
                            }
                        }
                    }
                };

                // Decode RGB part
                decode_into(&data[0..expected_size_opaque], &mut img_buf, false);

                if format == PtxFormat::Etc1A8 {
                    let size_uncompressed = expected_size_opaque + (width * height) as usize;
                    let size_compressed_no_header = expected_size_opaque * 2;
                    let size_compressed_with_header = size_compressed_no_header + 17;

                    // Determine alpha mode and header presence
                    let (use_compressed_alpha, has_header) = if let Some(af) = alpha_format {
                        if af == 100 {
                            if data.len() >= size_compressed_with_header {
                                (true, true)
                            } else {
                                (true, false)
                            }
                        } else {
                            (false, false)
                        }
                    } else {
                        // Heuristic based on size
                        // For small textures (e.g. 4x4), CompressedWithHeader (33) > Uncompressed (24)
                        // For large textures (e.g. 8x8), Uncompressed (96) > CompressedWithHeader (81)
                        // We should check the larger requirement first to avoid false positives from the smaller requirement matching the larger data.

                        if size_compressed_with_header > size_uncompressed {
                            // Small texture case: Check Compressed first
                            if data.len() >= size_compressed_with_header {
                                (true, true)
                            } else if data.len() >= size_uncompressed {
                                (false, false)
                            } else if data.len() >= size_compressed_no_header {
                                (true, false)
                            } else {
                                (false, false)
                            }
                        } else {
                            // Large texture case: Check Uncompressed first
                            if data.len() >= size_uncompressed {
                                (false, false)
                            } else if data.len() >= size_compressed_with_header {
                                (true, true)
                            } else if data.len() >= size_compressed_no_header {
                                (true, false)
                            } else {
                                (false, false)
                            }
                        }
                    };

                    if use_compressed_alpha {
                        if has_header {
                            // Check if this is Palette Alpha (17 byte header + 4bpp data)
                            // Size check: header (17) + (width * height) / 2
                            // Note: 4bpp alpha size is exactly same as ETC1 opaque size (0.5 bytes per pixel)
                            // So total alpha chunk size = 17 + expected_size_opaque

                            let alpha_chunk_size = data.len() - expected_size_opaque;
                            let expected_palette_size = 17 + expected_size_opaque;

                            if alpha_chunk_size == expected_palette_size {
                                // PALETTE ALPHA MODE
                                let offset = expected_size_opaque;
                                // Skip header (handled inside decode_palette_alpha or we pass slice)
                                // decode_palette_alpha expects the header to be present in the slice
                                let alpha_data = &data[offset..];
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
                            } else {
                                // Default back to ETC1 Compressed Alpha with Header (weird case loops 17 bytes offset)
                                // This was the previous "fix" or attempt.
                                // But if size doesn't match Palette, maybe it is ETC1?
                                let offset = expected_size_opaque + 17;
                                if offset + expected_size_opaque <= data.len() {
                                    decode_into(
                                        &data[offset..offset + expected_size_opaque],
                                        &mut img_buf,
                                        true,
                                    );
                                }
                            }
                        } else {
                            // Regular ETC1 Compressed Alpha (no header)
                            let offset = expected_size_opaque;
                            if offset + expected_size_opaque <= data.len() {
                                decode_into(
                                    &data[offset..offset + expected_size_opaque],
                                    &mut img_buf,
                                    true,
                                );
                            }
                        }
                    } else {
                        // Uncompressed Alpha (1 byte per pixel)
                        if data.len() >= size_uncompressed {
                            let alpha_data = &data[expected_size_opaque..];
                            for (i, pixel) in img_buf.pixels_mut().enumerate() {
                                pixel[3] = alpha_data[i];
                            }
                        }
                    }
                } else if format == PtxFormat::Etc1Palette {
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

                Ok(DynamicImage::ImageRgba8(img_buf))
            }
            _ => Err(RsbError::DeserializationError(format!(
                "Unsupported PTX format for decoding: {:?}",
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
        let img = image.to_rgba8();

        match format {
            PtxFormat::Rgba8888 => {
                // RGBA8888 (Direct copy)
                // Sen encodes RGBA8888 as RGBA buffer.
                Ok(img.into_raw())
            }
            PtxFormat::Rgba4444 => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for p in img.pixels() {
                    let r = (p[0] >> 4) as u16;
                    let g = (p[1] >> 4) as u16;
                    let b = (p[2] >> 4) as u16;
                    let a = (p[3] >> 4) as u16;
                    let val = (r << 12) | (g << 8) | (b << 4) | a;
                    out.extend_from_slice(&val.to_le_bytes());
                }
                Ok(out)
            }
            PtxFormat::Rgb565 => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for p in img.pixels() {
                    let r = (p[0] >> 3) as u16;
                    let g = (p[1] >> 2) as u16;
                    let b = (p[2] >> 3) as u16;
                    let val = (r << 11) | (g << 5) | b;
                    out.extend_from_slice(&val.to_le_bytes());
                }
                Ok(out)
            }
            PtxFormat::Rgba5551 => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for p in img.pixels() {
                    let r = (p[0] >> 3) as u16;
                    let g = (p[1] >> 3) as u16;
                    let b = (p[2] >> 3) as u16;
                    let a = if p[3] > 0 { 1 } else { 0 };
                    let val = (r << 11) | (g << 6) | (b << 1) | a;
                    out.extend_from_slice(&val.to_le_bytes());
                }
                Ok(out)
            }
            PtxFormat::Rgba4444Block => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;
                                if py < height && px < width {
                                    let p = img.get_pixel(px, py);
                                    let r = (p[0] >> 4) as u16;
                                    let g = (p[1] >> 4) as u16;
                                    let b = (p[2] >> 4) as u16;
                                    let a = (p[3] >> 4) as u16;
                                    let val = (r << 12) | (g << 8) | (b << 4) | a;
                                    out.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    out.extend_from_slice(&0u16.to_le_bytes());
                                }
                            }
                        }
                    }
                }
                Ok(out)
            }
            PtxFormat::Rgb565Block => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;
                                if py < height && px < width {
                                    let p = img.get_pixel(px, py);
                                    let r = (p[0] >> 3) as u16;
                                    let g = (p[1] >> 2) as u16;
                                    let b = (p[2] >> 3) as u16;
                                    let val = (r << 11) | (g << 5) | b;
                                    out.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    out.extend_from_slice(&0u16.to_le_bytes());
                                }
                            }
                        }
                    }
                }
                Ok(out)
            }
            PtxFormat::Rgba5551Block => {
                let mut out = Vec::with_capacity((width * height * 2) as usize);
                for y in (0..height).step_by(32) {
                    for x in (0..width).step_by(32) {
                        for j in 0..32 {
                            for k in 0..32 {
                                let py = y + j;
                                let px = x + k;
                                if py < height && px < width {
                                    let p = img.get_pixel(px, py);
                                    let r = (p[0] >> 3) as u16;
                                    let g = (p[1] >> 3) as u16;
                                    let b = (p[2] >> 3) as u16;
                                    let a = if p[3] > 0 { 1 } else { 0 };
                                    let val = (r << 11) | (g << 6) | (b << 1) | a;
                                    out.extend_from_slice(&val.to_le_bytes());
                                } else {
                                    out.extend_from_slice(&0u16.to_le_bytes());
                                }
                            }
                        }
                    }
                }
                Ok(out)
            }
            PtxFormat::Pvrtc4BppRgba | PtxFormat::Pvrtc4BppRgbaA8 => {
                // Ensure dimensions are POT and square? PVRTC usually requires POT or at least blocks.
                // Our implementation handles arbitrary size by blocks, but texture2ddecoder might have rules.
                // The C# port assumes blocks.

                let pixels: Vec<crate::color::Rgba32> = img
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
            PtxFormat::Etc1 | PtxFormat::Etc1A8 | PtxFormat::Etc1Palette => {
                let pixels: Vec<crate::color::Rgba32> = img
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

                if format == PtxFormat::Etc1A8 {
                    // Extract Alpha channel
                    let alpha_channel: Vec<u8> = img.pixels().map(|p| p[3]).collect();
                    out_data.append(&mut crate::codec::etc1::encode_alpha(
                        &alpha_channel,
                        width,
                        height,
                    ));
                } else if format == PtxFormat::Etc1Palette {
                    let alpha_channel: Vec<u8> = img.pixels().map(|p| p[3]).collect();
                    out_data.append(&mut crate::codec::etc1::encode_palette_alpha(
                        &alpha_channel,
                        width,
                        height,
                    ));
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
        let img_ios = PtxDecoder::decode(&bgra_data, 1, 1, 0, None, None, true)
            .expect("PowerVR decoding failed");
        let pixel_ios = *img_ios.to_rgba8().get_pixel(0, 0);
        assert_eq!(
            pixel_ios,
            Rgba([255, 0, 0, 255]),
            "PowerVR decoding (BGRA input) should result in Red pixel"
        );

        // Case 2: Default (Android) decoding of RGBA data
        // Flag = false. Expects RGBA input.
        let img_android = PtxDecoder::decode(&rgba_data, 1, 1, 0, None, None, false)
            .expect("Default decoding failed");
        let pixel_android = *img_android.to_rgba8().get_pixel(0, 0);
        assert_eq!(
            pixel_android,
            Rgba([255, 0, 0, 255]),
            "Default decoding (RGBA input) should result in Red pixel"
        );
    }

    #[test]
    fn test_ptx_encoding_sizes() {
        let width = 64;
        let height = 64;
        let img = DynamicImage::new_rgba8(width, height);

        // Rgba8888: 4 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba8888).unwrap();
        assert_eq!(data.len(), (width * height * 4) as usize);

        // Rgba4444: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba4444).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgb565: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgb565).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba5551: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba5551).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba4444Block: 2 bytes per pixel (no padding needed for 64x64)
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba4444Block).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgb565Block: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgb565Block).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba5551Block: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba5551Block).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Etc1: 0.5 bytes per pixel (4x4 block = 8 bytes)
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1).unwrap();
        assert_eq!(data.len(), (width * height / 2) as usize);

        // Etc1A8: Etc1 + 1 byte per pixel alpha
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1A8).unwrap();
        assert_eq!(data.len(), (width * height / 2 + width * height) as usize);

        // Etc1Palette: Etc1 + 1 header + 16 palette + 0.5 bytes per pixel alpha
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1Palette).unwrap();
        let expected = (width * height / 2) + 1 + 16 + (width * height / 2);
        assert_eq!(data.len(), expected as usize);
    }

    #[test]
    fn test_etc1_compressed_alpha() {
        let width = 4;
        let height = 4;
        let opaque_size = (width * height) / 2; // 8 bytes for 4x4
        let data_opaque = vec![0u8; opaque_size as usize]; // Dummy ETC1 block

        // Case 1: Header present (17 bytes)
        // 17 bytes header + 8 bytes (4x4 alpha block)
        let mut data_alpha = vec![0u8; 17];
        data_alpha.extend(vec![0u8; opaque_size as usize]);
        let mut data = data_opaque.clone();
        data.extend(data_alpha);

        let res = PtxDecoder::decode(&data, width, height, 147, None, Some(100), false);
        assert!(
            res.is_ok(),
            "ETC1 with compressed alpha and header should decode"
        );
    }

    #[test]
    fn test_etc1_alpha_legacy() {
        let width = 4;
        let height = 4;
        let opaque_size = (width * height) / 2;
        let data_opaque = vec![0u8; opaque_size as usize];
        let mut data_no_header = data_opaque.clone();
        data_no_header.extend(vec![0u8; opaque_size as usize]);
        let res = PtxDecoder::decode(&data_no_header, width, height, 147, None, Some(100), false);
        assert!(
            res.is_ok(),
            "ETC1 with compressed alpha (no header) should also decode if size matches exactly 2x"
        );
    }

    #[test]
    fn test_ambiguous_format_resolution() {
        // Test ID 147 (Etc1 vs Etc1A8)
        let width = 8;
        let height = 8;
        let opaque_size = (width * height) / 2; // 32
        let alpha_size = width * height; // 64

        // Case 1: Just opaque data -> Should be Etc1
        let data = vec![0u8; opaque_size as usize];
        // We can't check internal format easily, but we can check if it errors or decodes.
        // For Etc1, it should decode 4x4 opaque black (all zeros data -> somewhat valid ETC1 block).
        let res = PtxDecoder::decode(&data, width, height, 147, None, None, false);
        assert!(
            res.is_ok(),
            "ID 147 with opaque-only size should decode as ETC1"
        );

        // Case 2: Opaque + Alpha -> Should be Etc1A8
        // fill opaque part with some data
        let mut data_a8 = vec![0u8; (opaque_size + alpha_size) as usize];
        // Fill alpha with 255 (opaque) to distinguish from zero-init (transparent)
        for i in opaque_size as usize..data_a8.len() {
            data_a8[i] = 255;
        }
        let res = PtxDecoder::decode(&data_a8, width, height, 147, None, None, false);
        assert!(
            res.is_ok(),
            "ID 147 with alpha size should decode as Etc1A8"
        );
        // Check a pixel to see if alpha was read (opaque)
        let img = res.unwrap();
        assert_eq!(
            img.to_rgba8().get_pixel(0, 0)[3],
            255,
            "Etc1A8 should read alpha channel"
        );

        // Test ID 30 (Pvrtc vs Etc1Palette)
        // 8x8 PVRTC 4bpp = 32 bytes.

        // Case 3: PVRTC size -> Pvrtc4Bpp
        let pvrtc_data = vec![0u8; 32];
        let res = PtxDecoder::decode(&pvrtc_data, width, height, 30, None, None, false);
        assert!(res.is_ok(), "ID 30 with PVRTC size should decode as PVRTC");

        // Case 4: Larger size -> Etc1Palette
        // Etc1Palette size for 8x8: 32 (opaque) + 1 (header) + 16 (palette) + 32 (alpha data) = 81 bytes.
        let mut palette_data = vec![0u8; 81];
        // Set palette header to 0x10 (16 colors)
        palette_data[32] = 0x10;
        let res = PtxDecoder::decode(&palette_data, width, height, 30, None, None, false);
        assert!(
            res.is_ok(),
            "ID 30 with larger size should decode as Etc1Palette"
        );
        // Case 5: ETC1 + Compressed Alpha (Double Size + Header)
        // The original test assumed exact 2x size. Let's update it to use the referenced 1200_00.PTX structure
        let width = 4;
        let height = 4;
        let opaque_size = 8;
        let data_compressed_alpha = vec![0u8; (opaque_size * 2) + 17];
        let res = PtxDecoder::decode(
            &data_compressed_alpha,
            width,
            height,
            147,
            None,
            Some(100), // Explicitly passing alpha_format
            false,
        );
        assert!(
            res.is_ok(),
            "ID 147 with 2x size + 17 bytes should decode as ETC1 + Compressed Alpha with Header"
        );
    }
}
