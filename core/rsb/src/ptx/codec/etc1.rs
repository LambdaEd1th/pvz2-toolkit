use crate::ptx::color::Rgba32;

pub const ETC1_MODIFIERS: [[i32; 2]; 8] = [
    [2, 8],
    [5, 17],
    [9, 29],
    [13, 42],
    [18, 60],
    [24, 80],
    [33, 106],
    [47, 183],
];

pub fn vertical_etc1(colors: &[Rgba32; 16]) -> u64 {
    gen_vertical(colors)
}

pub fn gen_etc1(colors: &[Rgba32; 16]) -> u64 {
    let horizontal = gen_horizontal(colors);
    let vertical = gen_vertical(colors);

    let mut source_color = [Rgba32::new(0, 0, 0, 0); 16];

    decode_etc1_block(horizontal, &mut source_color);
    let horizontal_score = get_score(colors, &source_color);

    decode_etc1_block(vertical, &mut source_color);
    let vertical_score = get_score(colors, &source_color);

    if horizontal_score < vertical_score {
        horizontal
    } else {
        vertical
    }
}

// Alias for gen_etc1 to match encoder.rs expectation
pub fn encode_etc1_block(colors: &[Rgba32; 16]) -> u64 {
    gen_etc1(colors)
}

pub fn encode_etc1_alpha_block(colors: &[Rgba32; 16]) -> u64 {
    // For alpha, we can just use the standard ETC1 encoder on the alpha channel treated as grayscale
    // But we need to convert alpha to RGB (check if R==G==B==A)
    // The encoder expects Rgba32, so we can pass it as is if it's grayscale
    // Or we force it to grayscale?
    // The caller ensures colors are (A,A,A,255)
    gen_etc1(colors)
}

pub fn decode_etc1_block(temp: u64, result: &mut [Rgba32]) {
    let diffbit = ((temp >> 33) & 1) == 1;
    let flipbit = ((temp >> 32) & 1) == 1;
    let r1;
    let r2;
    let g1;
    let g2;
    let b1;
    let b2;

    if diffbit {
        let mut r = ((temp >> 59) & 0x1F) as i32;
        let mut g = ((temp >> 51) & 0x1F) as i32;
        let mut b = ((temp >> 43) & 0x1F) as i32;

        r1 = (r << 3) | ((r & 0x1C) >> 2);
        g1 = (g << 3) | ((g & 0x1C) >> 2);
        b1 = (b << 3) | ((b & 0x1C) >> 2);

        // Sign extension logic from C#
        // r += (int)((temp >> 56) & 0x7) << 29 >> 29;
        // In Rust, we cast to i32, shift, then arithmetic shift back to propagate sign
        let dr = (((temp >> 56) & 0x7) as i32) << 29 >> 29;
        let dg = (((temp >> 48) & 0x7) as i32) << 29 >> 29;
        let db = (((temp >> 40) & 0x7) as i32) << 29 >> 29;

        r += dr;
        g += dg;
        b += db;

        r2 = (r << 3) | ((r & 0x1C) >> 2);
        g2 = (g << 3) | ((g & 0x1C) >> 2);
        b2 = (b << 3) | ((b & 0x1C) >> 2);
    } else {
        r1 = (((temp >> 60) & 0xF) * 0x11) as i32;
        g1 = (((temp >> 52) & 0xF) * 0x11) as i32;
        b1 = (((temp >> 44) & 0xF) * 0x11) as i32;
        r2 = (((temp >> 56) & 0xF) * 0x11) as i32;
        g2 = (((temp >> 48) & 0xF) * 0x11) as i32;
        b2 = (((temp >> 40) & 0xF) * 0x11) as i32;
    }

    let table1 = ((temp >> 37) & 0x7) as usize;
    let table2 = ((temp >> 34) & 0x7) as usize;

    for i in 0..4 {
        for j in 0..4 {
            // Note: ETC1 pixel selectors are usually Column-Major (x*4 + y).
            // i is x, j is y.
            let val = ((temp >> ((i << 2) | j)) & 0x1) as usize;
            let neg = ((temp >> (((i << 2) | j) + 16)) & 0x1) == 1;

            let add;
            // flipbit = 1: Top/Bottom (Split Y). Subblock 1 if y < 2.
            // flipbit = 0: Left/Right (Split X). Subblock 1 if x < 2.
            if (flipbit && j < 2) || (!flipbit && i < 2) {
                add = ETC1_MODIFIERS[table1][val] * if neg { -1 } else { 1 };
                result[(i << 2) | j] = Rgba32::new(
                    color_clamp(r1 + add),
                    color_clamp(g1 + add),
                    color_clamp(b1 + add),
                    255,
                );
            } else {
                add = ETC1_MODIFIERS[table2][val] * if neg { -1 } else { 1 };
                result[(i << 2) | j] = Rgba32::new(
                    color_clamp(r2 + add),
                    color_clamp(g2 + add),
                    color_clamp(b2 + add),
                    255,
                );
            }
        }
    }
}

use crate::error::Result;
use crate::error::RsbError;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer};

pub fn decode_etc1(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
    let mut img_buf = ImageBuffer::new(width, height);
    let mut offset = 0;

    // ETC1 blocks are 4x4 pixels, 8 bytes
    for y in (0..height).step_by(4) {
        for x in (0..width).step_by(4) {
            if offset + 8 > data.len() {
                return Err(RsbError::DeserializationError(
                    "Insufficient data for ETC1".into(),
                ));
            }

            // Read 64-bit block (Big Endian)
            let block = u64::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            offset += 8;

            let mut decoded_pixels = [Rgba32::default(); 16];
            decode_etc1_block(block, &mut decoded_pixels);

            for dy in 0..4 {
                for dx in 0..4 {
                    if x + dx < width && y + dy < height {
                        let p = decoded_pixels[(dx * 4 + dy) as usize];
                        img_buf.put_pixel(x + dx, y + dy, p.to_pixel());
                    }
                }
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img_buf))
}

pub fn decode_etc1_a8(
    data_color: &[u8],
    data_alpha: &[u8],
    width: u32,
    height: u32,
    compressed_alpha: bool,
) -> Result<DynamicImage> {
    let mut img = decode_etc1(data_color, width, height)?;

    if compressed_alpha {
        // Alpha encoded as ETC1
        let alpha_img = decode_etc1(data_alpha, width, height)?;

        // Combine: use alpha_img's green channel as alpha
        for y in 0..height {
            for x in 0..width {
                let mut p_color = img.get_pixel(x, y);
                let p_alpha = alpha_img.get_pixel(x, y);
                // Assuming ETC1 encoding of grayscale places value in G (and R, B)
                p_color[3] = p_alpha[1];
                img.put_pixel(x, y, p_color);
            }
        }
    } else {
        // Uncompressed Alpha (1 byte per pixel)
        if data_alpha.len() < (width * height) as usize {
            return Err(RsbError::DeserializationError(
                "Insufficient alpha data".into(),
            ));
        }

        for y in 0..height {
            for x in 0..width {
                let mut p_color = img.get_pixel(x, y);
                let a = data_alpha[(y * width + x) as usize];
                p_color[3] = a;
                img.put_pixel(x, y, p_color);
            }
        }
    }

    Ok(img)
}

pub fn decode_palette_alpha(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
    // This is essentially PtxDecoder logic but specific for this format.
    // However, it returns a full image which PtxDecoder expects.
    // The previous implementation returned specific alpha values.
    // We need to return an image, BUT Palette Alpha format is standalone?
    // NO, PtxFormat::Etc1Palette is (ETC1 Data + Alpha Data).
    // The previous implementation of `decode_palette_alpha` only returned Alpha bytes?
    // Let's check `lib.rs` (original) or `pvrtc` logic.
    // Ah, `lib.rs` called `decode_palette_alpha` inside `PtxDecoder::decode`.
    // It seems `Etc1Palette` format implies:
    // [ETC1 Data (Opaque)] [Palette Alpha Data]
    // So we first decode ETC1, then decode alpha and apply it.

    let opaque_size = (width * height) as usize / 2;
    if data.len() < opaque_size {
        return Err(RsbError::DeserializationError(
            "Insufficient data for ETC1 Palette".into(),
        ));
    }

    let etc1_data = &data[..opaque_size];
    let alpha_data = &data[opaque_size..];

    let mut img = decode_etc1(etc1_data, width, height)?;

    // Now decode alpha
    // The previous helper `decode_palette_alpha` returns Vec<u8> (alpha values).
    // Let's rename the helper to `decode_palette_alpha_values` and call it.

    let alphas = decode_palette_alpha_values(alpha_data, (width * height) as usize)
        .map_err(RsbError::DeserializationError)?;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if idx < alphas.len() {
                let mut p = img.get_pixel(x, y);
                p[3] = alphas[idx];
                img.put_pixel(x, y, p);
            }
        }
    }

    Ok(img)
}

pub fn decode_palette_alpha_values(
    data: &[u8],
    num_pixels: usize,
) -> std::result::Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("Empty alpha data".to_string());
    }

    let mut cursor = 0;

    if cursor >= data.len() {
        return Err("Insufficient alpha data".to_string());
    }
    let num = data[cursor] as usize;
    cursor += 1;

    let mut index_table = Vec::with_capacity(if num == 0 { 2 } else { num });
    let bit_depth = if num == 0 {
        index_table.push(0x00);
        index_table.push(0xFF);
        1
    } else {
        if cursor + num > data.len() {
            return Err(format!(
                "Insufficient alpha data for palette: expected {}, got {}",
                num,
                data.len() - cursor
            ));
        }
        for _ in 0..num {
            let p_byte = data[cursor];
            cursor += 1;
            index_table.push((p_byte << 4) | p_byte);
        }

        let mut table_size = 2;
        let mut bd = 1;
        while num > table_size {
            table_size *= 2;
            bd += 1;
        }
        bd
    };

    let mut alpha_values = Vec::with_capacity(num_pixels);
    let mut bit_position = 0;
    let mut buffer = 0u8;

    // Helper closure to read one bit
    // Note: Rust closures can't easily capture mutable borrows of cursor/buffer/bit_position in a way that allows re-borrowing nicely inside loop if not careful,
    // but here we can just write loop inline or use a struct.
    // C#:
    // int readOneBit() {
    //    if (bitPostion == 0) { buffer = readBytes(); }
    //    bitPostion = (bitPostion + 7) & 7;
    //    return (buffer >> bitPostion) & 1;
    // }

    for _ in 0..num_pixels {
        let mut ans = 0;
        // readBits(bit_depth) -> Reads MSB first (i loops from bits-1 down to 0)
        for i in (0..bit_depth).rev() {
            if bit_position == 0 {
                if cursor >= data.len() {
                    // Should we error or pad? C# probably throws or returns 0.
                    // Let's assume 0.
                    buffer = 0; // Prevent crash
                } else {
                    buffer = data[cursor];
                    cursor += 1;
                }
            }
            bit_position = (bit_position + 7) & 7;
            let bit = (buffer >> bit_position) & 1;
            ans |= (bit as usize) << i;
        }

        if ans >= index_table.len() {
            // Should not happen if logic matches, but for safety
            alpha_values.push(index_table[0]);
        } else {
            alpha_values.push(index_table[ans]);
        }
    }

    Ok(alpha_values)
}

fn get_score(original: &[Rgba32], encode: &[Rgba32]) -> i32 {
    let mut diff = 0;
    for i in 0..16 {
        diff += (encode[i].r as i32 - original[i].r as i32).abs();
        diff += (encode[i].g as i32 - original[i].g as i32).abs();
        diff += (encode[i].b as i32 - original[i].b as i32).abs();
    }
    diff
}

fn set_flip_mode(data: &mut u64, mode: bool) {
    *data &= !(1u64 << 32);
    *data |= (if mode { 1u64 } else { 0u64 }) << 32;
}

fn set_diff_mode(data: &mut u64, mode: bool) {
    *data &= !(1u64 << 33);
    *data |= (if mode { 1u64 } else { 0u64 }) << 33;
}

fn get_left_colors(pixels: &[Rgba32; 16]) -> [Rgba32; 8] {
    let mut left = [Rgba32::new(0, 0, 0, 0); 8];
    for y in 0..4 {
        for x in 0..2 {
            left[y * 2 + x] = pixels[y * 4 + x];
        }
    }
    left
}

fn get_right_colors(pixels: &[Rgba32; 16]) -> [Rgba32; 8] {
    let mut right = [Rgba32::new(0, 0, 0, 0); 8];
    for y in 0..4 {
        for x in 2..4 {
            right[y * 2 + x - 2] = pixels[y * 4 + x];
        }
    }
    right
}

fn get_top_colors(pixels: &[Rgba32; 16]) -> [Rgba32; 8] {
    let mut top = [Rgba32::new(0, 0, 0, 0); 8];
    for y in 0..2 {
        for x in 0..4 {
            top[y * 4 + x] = pixels[y * 4 + x];
        }
    }
    top
}

fn get_bottom_colors(pixels: &[Rgba32; 16]) -> [Rgba32; 8] {
    let mut bottom = [Rgba32::new(0, 0, 0, 0); 8];
    for y in 2..4 {
        for x in 0..4 {
            bottom[(y - 2) * 4 + x] = pixels[y * 4 + x];
        }
    }
    bottom
}

fn gen_horizontal(colors: &[Rgba32; 16]) -> u64 {
    let mut data = 0;
    set_flip_mode(&mut data, false);

    let left = get_left_colors(colors);
    let mut base_c1 = Rgba32::default();
    let mod1 = gen_modifier(&mut base_c1, &left);
    set_table1(&mut data, mod1);
    gen_pix_diff(&mut data, &left, base_c1, mod1, 0, 2, 0, 4);

    let right = get_right_colors(colors);
    let mut base_c2 = Rgba32::default();
    let mod2 = gen_modifier(&mut base_c2, &right);
    set_table2(&mut data, mod2);
    gen_pix_diff(&mut data, &right, base_c2, mod2, 2, 4, 0, 4);

    set_base_colors(&mut data, base_c1, base_c2);
    data
}

fn gen_vertical(colors: &[Rgba32; 16]) -> u64 {
    let mut data = 0;
    set_flip_mode(&mut data, true);

    let top = get_top_colors(colors);
    let mut base_c1 = Rgba32::default();
    let mod1 = gen_modifier(&mut base_c1, &top);
    set_table1(&mut data, mod1);
    gen_pix_diff(&mut data, &top, base_c1, mod1, 0, 4, 0, 2);

    let bottom = get_bottom_colors(colors);
    let mut base_c2 = Rgba32::default();
    let mod2 = gen_modifier(&mut base_c2, &bottom);
    set_table2(&mut data, mod2);
    gen_pix_diff(&mut data, &bottom, base_c2, mod2, 0, 4, 2, 4);

    set_base_colors(&mut data, base_c1, base_c2);
    data
}

fn set_base_colors(data: &mut u64, color1: Rgba32, color2: Rgba32) {
    let r1 = color1.r as i32;
    let g1 = color1.g as i32;
    let b1 = color1.b as i32;
    let r2 = color2.r as i32;
    let g2 = color2.g as i32;
    let b2 = color2.b as i32;

    let r_diff = (r2 - r1) / 8;
    let g_diff = (g2 - g1) / 8;
    let b_diff = (b2 - b1) / 8;

    if r_diff > -4 && r_diff < 3 && g_diff > -4 && g_diff < 3 && b_diff > -4 && b_diff < 3 {
        set_diff_mode(data, true);

        let r1_5 = r1 / 8;
        let g1_5 = g1 / 8;
        let b1_5 = b1 / 8;

        *data |= (r1_5 as u64) << 59;
        *data |= (g1_5 as u64) << 51;
        *data |= (b1_5 as u64) << 43;

        *data |= ((r_diff) as u64 & 0x7) << 56;
        *data |= ((g_diff) as u64 & 0x7) << 48;
        *data |= ((b_diff) as u64 & 0x7) << 40;
    } else {
        *data |= ((r1 / 0x11) as u64) << 60;
        *data |= ((g1 / 0x11) as u64) << 52;
        *data |= ((b1 / 0x11) as u64) << 44;

        *data |= ((r2 / 0x11) as u64) << 56;
        *data |= ((g2 / 0x11) as u64) << 48;
        *data |= ((b2 / 0x11) as u64) << 40;
    }
}

#[allow(clippy::too_many_arguments)]
fn gen_pix_diff(
    data: &mut u64,
    pixels: &[Rgba32],
    base_color: Rgba32,
    modifier: usize,
    x_offs: usize,
    x_end: usize,
    y_offs: usize,
    y_end: usize,
) {
    let base_mean = (base_color.r as i32 + base_color.g as i32 + base_color.b as i32) / 3;
    let mut i = 0;

    for yy in y_offs..y_end {
        for xx in x_offs..x_end {
            let diff =
                ((pixels[i].r as i32 + pixels[i].g as i32 + pixels[i].b as i32) / 3) - base_mean;

            if diff < 0 {
                *data |= 1u64 << (xx * 4 + yy + 16);
            }
            let tbl_diff1 = diff.abs() - ETC1_MODIFIERS[modifier][0];
            let tbl_diff2 = diff.abs() - ETC1_MODIFIERS[modifier][1];

            if tbl_diff2.abs() < tbl_diff1.abs() {
                *data |= 1u64 << (xx * 4 + yy);
            }
            i += 1;
        }
    }
}

fn gen_modifier(base_color: &mut Rgba32, pixels: &[Rgba32]) -> usize {
    let mut max = Rgba32::new(255, 255, 255, 255);
    let mut min = Rgba32::new(0, 0, 0, 255);
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pixel in pixels.iter().take(8) {
        if pixel.a == 0 {
            continue;
        }
        let y = (pixel.r as i32 + pixel.g as i32 + pixel.b as i32) / 3;
        if y > max_y {
            max_y = y;
            max = *pixel;
        }
        if y < min_y {
            min_y = y;
            min = *pixel;
        }
    }

    let diff_mean = (max.r as i32 - min.r as i32 + max.g as i32 - min.g as i32 + max.b as i32
        - min.b as i32)
        / 3;

    let mut mod_diff = i32::MAX;
    let mut modifier = 0; // Default if -1
    let mut mode = -1;

    for (i, modifier_vals) in ETC1_MODIFIERS.iter().enumerate() {
        let ss = modifier_vals[0] * 2;
        let sb = modifier_vals[0] + modifier_vals[1];
        let bb = modifier_vals[1] * 2;

        let ss = if ss > 255 { 255 } else { ss };
        let sb = if sb > 255 { 255 } else { sb };
        let bb = if bb > 255 { 255 } else { bb };

        if (diff_mean - ss).abs() < mod_diff {
            mod_diff = (diff_mean - ss).abs();
            modifier = i;
            mode = 0;
        }
        if (diff_mean - sb).abs() < mod_diff {
            mod_diff = (diff_mean - sb).abs();
            modifier = i;
            mode = 1;
        }
        if (diff_mean - bb).abs() < mod_diff {
            mod_diff = (diff_mean - bb).abs();
            modifier = i;
            mode = 2;
        }
    }

    if mode == 1 {
        let div1 = ETC1_MODIFIERS[modifier][0] as f32 / ETC1_MODIFIERS[modifier][1] as f32;
        let div2 = 1.0 - div1;
        *base_color = Rgba32::new(
            color_clamp_f(min.r as f32 * div1 + max.r as f32 * div2),
            color_clamp_f(min.g as f32 * div1 + max.g as f32 * div2),
            color_clamp_f(min.b as f32 * div1 + max.b as f32 * div2),
            255,
        );
    } else {
        *base_color = Rgba32::new(
            ((min.r as i32 + max.r as i32) >> 1) as u8,
            ((min.g as i32 + max.g as i32) >> 1) as u8,
            ((min.b as i32 + max.b as i32) >> 1) as u8,
            255,
        );
    }

    modifier
}

fn set_table1(data: &mut u64, table: usize) {
    *data &= !(7u64 << 37);
    *data |= ((table & 0x7) as u64) << 37;
}

fn set_table2(data: &mut u64, table: usize) {
    *data &= !(7u64 << 34);
    *data |= ((table & 0x7) as u64) << 34;
}

pub fn color_clamp(color: i32) -> u8 {
    if color > 255 {
        return 255;
    }
    if color < 0 {
        return 0;
    }
    color as u8
}

fn color_clamp_f(color: f32) -> u8 {
    let c = color as i32;
    if c > 255 {
        return 255;
    }
    if c < 0 {
        return 0;
    }
    c as u8
}

pub fn encode_alpha(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height) as usize);
    out.extend_from_slice(data);
    out
}

pub fn encode_palette_alpha(image: &DynamicImage) -> Result<Vec<u8>> {
    // 1. Encode image as ETC1 (RGB)
    // 2. Extract alpha channel and encode as Palette

    let width = image.width();
    let height = image.height();
    let num_pixels = (width * height) as usize;

    // Pass 1: RGB -> ETC1
    // We already have code in encoder.rs for this, but to reuse:
    // We need to call encode_etc1 (but that expects image too)
    // Or we manually do it here.

    let mut data = Vec::new(); // Final data

    // --- ETC1 Encode ---
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);

    // RGB Data
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
            let encoded = encode_etc1_block(&block_pixels);
            data.extend_from_slice(&encoded.to_be_bytes());
        }
    }

    // --- Palette Alpha Encode ---

    // Collect alpha values
    let mut alphas = Vec::with_capacity(num_pixels);
    for y in 0..height {
        for x in 0..width {
            alphas.push(image.get_pixel(x, y)[3]);
        }
    }

    // Header: num = 16 (0x10)
    data.push(0x10);
    // Palette: 0..15 -> 0x00, 0x11, ... 0xFF
    for i in 0..16 {
        data.push(i as u8);
    }

    // Data: 4-bit indices (val >> 4)
    let mut writer = BitWriter::new();
    for &a in &alphas {
        let val = a >> 4;
        writer.write_bits(val as u32, 4);
    }
    data.append(&mut writer.flush());

    Ok(data)
}

struct BitWriter {
    buffer: u8,
    bit_pos: u8,
    bytes: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            buffer: 0,
            bit_pos: 0,
            bytes: Vec::new(),
        }
    }

    fn write_bits(&mut self, val: u32, bits: u8) {
        // Decoder reads MSB first (i = bits-1 down to 0) from stream LSB first
        // So we must write MSB of val to next bit position in buffer
        for i in (0..bits).rev() {
            let bit = (val >> i) & 1;
            self.buffer |= (bit as u8) << self.bit_pos;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bytes.push(self.buffer);
                self.buffer = 0;
                self.bit_pos = 0;
            }
        }
    }

    fn flush(&mut self) -> Vec<u8> {
        let mut res = std::mem::take(&mut self.bytes);
        if self.bit_pos > 0 {
            res.push(self.buffer);
        }
        res
    }
}
