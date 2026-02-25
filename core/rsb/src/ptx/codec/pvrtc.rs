use crate::error::{Result, RsbError};
use crate::ptx::color::{ColorRGBA, Rgba32};
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer};

const MORTON_TABLE: [i32; 256] = [
    0x0000, 0x0001, 0x0004, 0x0005, 0x0010, 0x0011, 0x0014, 0x0015, 0x0040, 0x0041, 0x0044, 0x0045,
    0x0050, 0x0051, 0x0054, 0x0055, 0x0100, 0x0101, 0x0104, 0x0105, 0x0110, 0x0111, 0x0114, 0x0115,
    0x0140, 0x0141, 0x0144, 0x0145, 0x0150, 0x0151, 0x0154, 0x0155, 0x0400, 0x0401, 0x0404, 0x0405,
    0x0410, 0x0411, 0x0414, 0x0415, 0x0440, 0x0441, 0x0444, 0x0445, 0x0450, 0x0451, 0x0454, 0x0455,
    0x0500, 0x0501, 0x0504, 0x0505, 0x0510, 0x0511, 0x0514, 0x0515, 0x0540, 0x0541, 0x0544, 0x0545,
    0x0550, 0x0551, 0x0554, 0x0555, 0x1000, 0x1001, 0x1004, 0x1005, 0x1010, 0x1011, 0x1014, 0x1015,
    0x1040, 0x1041, 0x1044, 0x1045, 0x1050, 0x1051, 0x1054, 0x1055, 0x1100, 0x1101, 0x1104, 0x1105,
    0x1110, 0x1111, 0x1114, 0x1115, 0x1140, 0x1141, 0x1144, 0x1145, 0x1150, 0x1151, 0x1154, 0x1155,
    0x1400, 0x1401, 0x1404, 0x1405, 0x1410, 0x1411, 0x1414, 0x1415, 0x1440, 0x1441, 0x1444, 0x1445,
    0x1450, 0x1451, 0x1454, 0x1455, 0x1500, 0x1501, 0x1504, 0x1505, 0x1510, 0x1511, 0x1514, 0x1515,
    0x1540, 0x1541, 0x1544, 0x1545, 0x1550, 0x1551, 0x1554, 0x1555, 0x4000, 0x4001, 0x4004, 0x4005,
    0x4010, 0x4011, 0x4014, 0x4015, 0x4040, 0x4041, 0x4044, 0x4045, 0x4050, 0x4051, 0x4054, 0x4055,
    0x4100, 0x4101, 0x4104, 0x4105, 0x4110, 0x4111, 0x4114, 0x4115, 0x4140, 0x4141, 0x4144, 0x4145,
    0x4150, 0x4151, 0x4154, 0x4155, 0x4400, 0x4401, 0x4404, 0x4405, 0x4410, 0x4411, 0x4414, 0x4415,
    0x4440, 0x4441, 0x4444, 0x4445, 0x4450, 0x4451, 0x4454, 0x4455, 0x4500, 0x4501, 0x4504, 0x4505,
    0x4510, 0x4511, 0x4514, 0x4515, 0x4540, 0x4541, 0x4544, 0x4545, 0x4550, 0x4551, 0x4554, 0x4555,
    0x5000, 0x5001, 0x5004, 0x5005, 0x5010, 0x5011, 0x5014, 0x5015, 0x5040, 0x5041, 0x5044, 0x5045,
    0x5050, 0x5051, 0x5054, 0x5055, 0x5100, 0x5101, 0x5104, 0x5105, 0x5110, 0x5111, 0x5114, 0x5115,
    0x5140, 0x5141, 0x5144, 0x5145, 0x5150, 0x5151, 0x5154, 0x5155, 0x5400, 0x5401, 0x5404, 0x5405,
    0x5410, 0x5411, 0x5414, 0x5415, 0x5440, 0x5441, 0x5444, 0x5445, 0x5450, 0x5451, 0x5454, 0x5455,
    0x5500, 0x5501, 0x5504, 0x5505, 0x5510, 0x5511, 0x5514, 0x5515, 0x5540, 0x5541, 0x5544, 0x5545,
    0x5550, 0x5551, 0x5554, 0x5555,
];

const BILINEAR_FACTORS: [[u8; 4]; 16] = [
    [4, 4, 4, 4],
    [2, 6, 2, 6],
    [8, 0, 8, 0],
    [6, 2, 6, 2],
    [2, 2, 6, 6],
    [1, 3, 3, 9],
    [4, 0, 12, 0],
    [3, 1, 9, 3],
    [8, 8, 0, 0],
    [4, 12, 0, 0],
    [16, 0, 0, 0],
    [12, 4, 0, 0],
    [6, 6, 2, 2],
    [3, 9, 1, 3],
    [12, 0, 4, 0],
    [9, 3, 3, 1],
];

const WEIGHTS: [u8; 32] = [
    8, 0, 8, 0, 5, 3, 5, 3, 3, 5, 3, 5, 0, 8, 0, 8, 8, 0, 8, 0, 4, 4, 4, 4, 4, 4, 0, 0, 0, 8, 0, 8,
];

#[derive(Clone, Copy, Default)]
pub struct PvrTcPacket {
    pub pvr_tc_word: u64,
}

impl PvrTcPacket {
    pub fn new(word: u64) -> Self {
        Self { pvr_tc_word: word }
    }

    pub fn set_modulation_data(&mut self, value: u32) {
        self.pvr_tc_word &= !0xFFFFFFFF;
        self.pvr_tc_word |= value as u64;
    }

    pub fn modulation_data(&self) -> u32 {
        (self.pvr_tc_word & 0xFFFFFFFF) as u32
    }

    pub fn set_use_punchthrough_alpha(&mut self, value: bool) {
        self.pvr_tc_word |= (if value { 1u64 } else { 0u64 }) << 32;
    }

    pub fn use_punchthrough_alpha(&self) -> bool {
        ((self.pvr_tc_word >> 32) & 0b1) == 1
    }

    pub fn set_color_a(&mut self, value: i32) {
        self.pvr_tc_word |= (value as u64 & 0b11111111111111) << 33;
    }

    pub fn color_a(&self) -> i32 {
        ((self.pvr_tc_word >> 33) & 0b11111111111111) as i32
    }

    pub fn set_color_a_is_opaque(&mut self, value: bool) {
        self.pvr_tc_word |= (if value { 1u64 } else { 0u64 }) << 47;
    }

    pub fn color_a_is_opaque(&self) -> bool {
        ((self.pvr_tc_word >> 47) & 0b1) == 1
    }

    pub fn set_color_b(&mut self, value: i32) {
        self.pvr_tc_word |= (value as u64 & 0b111111111111111) << 48;
    }

    pub fn color_b(&self) -> i32 {
        ((self.pvr_tc_word >> 48) & 0b111111111111111) as i32
    }

    pub fn set_color_b_is_opaque(&mut self, value: bool) {
        self.pvr_tc_word |= (if value { 1u64 } else { 0u64 }) << 63;
    }

    pub fn color_b_is_opaque(&self) -> bool {
        (self.pvr_tc_word >> 63) == 1
    }

    pub fn get_color_a_rgba(&self) -> ColorRGBA {
        let color_a = self.color_a();
        if self.color_a_is_opaque() {
            let r = color_a >> 9;
            let g = (color_a >> 4) & 0x1F;
            let b = color_a & 0xF;
            ColorRGBA::new((r << 3) | (r >> 2), (g << 3) | (g >> 2), (b << 4) | b, 255)
        } else {
            let a = (color_a >> 11) & 0x7;
            let r = (color_a >> 7) & 0xF;
            let g = (color_a >> 3) & 0xF;
            let b = color_a & 0x7;
            ColorRGBA::new(
                (r << 4) | r,
                (g << 4) | g,
                (b << 5) | (b << 2) | (b >> 1),
                (a << 5) | (a << 2) | (a >> 1),
            )
        }
    }

    pub fn get_color_b_rgba(&self) -> ColorRGBA {
        let color_b = self.color_b();
        if self.color_b_is_opaque() {
            let r = color_b >> 10;
            let g = (color_b >> 5) & 0x1F;
            let b = color_b & 0x1F;
            ColorRGBA::new(
                (r << 3) | (r >> 2),
                (g << 3) | (g >> 2),
                (b << 3) | (b >> 2),
                255,
            )
        } else {
            let a = (color_b >> 12) & 0x7;
            let r = (color_b >> 8) & 0xF;
            let g = (color_b >> 4) & 0xF;
            let b = color_b & 0xF;
            ColorRGBA::new(
                (r << 4) | r,
                (g << 4) | g,
                (b << 4) | b,
                (a << 5) | (a << 2) | (a >> 1),
            )
        }
    }
}

pub fn decode_4bpp(packets: &[PvrTcPacket], width: i32) -> Vec<Rgba32> {
    let blocks = width >> 2;
    let block_mask = blocks - 1;
    let mut result = vec![Rgba32::default(); (width * width) as usize];

    for y in 0..blocks {
        for x in 0..blocks {
            let packet = packets[get_morton_number(x, y)];
            let mut mod_data = packet.modulation_data();

            let weight_index = if packet.use_punchthrough_alpha() {
                16
            } else {
                0
            };
            let mut factor_index = 0;

            for py in 0..4 {
                let y_offset = if py < 2 { -1 } else { 0 };
                let y0 = (y + y_offset) & block_mask;
                let y1 = (y0 + 1) & block_mask;

                for px in 0..4 {
                    let factor = BILINEAR_FACTORS[factor_index];
                    let x_offset = if px < 2 { -1 } else { 0 };
                    let x0 = (x + x_offset) & block_mask;
                    let x1 = (x0 + 1) & block_mask;

                    let p0 = packets[get_morton_number(x0, y0)];
                    let p1 = packets[get_morton_number(x1, y0)];
                    let p2 = packets[get_morton_number(x0, y1)];
                    let p3 = packets[get_morton_number(x1, y1)];

                    let ca = p0.get_color_a_rgba() * factor[0] as i32
                        + p1.get_color_a_rgba() * factor[1] as i32
                        + p2.get_color_a_rgba() * factor[2] as i32
                        + p3.get_color_a_rgba() * factor[3] as i32;

                    let cb = p0.get_color_b_rgba() * factor[0] as i32
                        + p1.get_color_b_rgba() * factor[1] as i32
                        + p2.get_color_b_rgba() * factor[2] as i32
                        + p3.get_color_b_rgba() * factor[3] as i32;

                    let index = weight_index + (((mod_data as i32) & 0b11) << 2) as usize;

                    let r = (ca.r * WEIGHTS[index] as i32 + cb.r * WEIGHTS[index + 1] as i32) >> 7;
                    let g = (ca.g * WEIGHTS[index] as i32 + cb.g * WEIGHTS[index + 1] as i32) >> 7;
                    let b = (ca.b * WEIGHTS[index] as i32 + cb.b * WEIGHTS[index + 1] as i32) >> 7;
                    let a =
                        (ca.a * WEIGHTS[index + 2] as i32 + cb.a * WEIGHTS[index + 3] as i32) >> 7;

                    let result_idx = ((py + (y << 2)) * width + px + (x << 2)) as usize;
                    result[result_idx] = Rgba32::new(r as u8, g as u8, b as u8, a as u8);

                    mod_data >>= 2;
                    factor_index += 1;
                }
            }
        }
    }
    result
}

fn get_morton_number(x: i32, y: i32) -> usize {
    ((MORTON_TABLE[(x >> 8) as usize] << 17)
        | (MORTON_TABLE[(y >> 8) as usize] << 16)
        | (MORTON_TABLE[(x & 0xFF) as usize] << 1)
        | MORTON_TABLE[(y & 0xFF) as usize]) as usize
}

pub fn encode_rgba_4bpp(_colors: &[Rgba32], width: i32) -> Vec<PvrTcPacket> {
    let blocks = width / 4;
    let packet_count = (blocks * blocks) as usize;
    let mut packets = Vec::with_capacity(packet_count);
    for _ in 0..packet_count {
        packets.push(PvrTcPacket::new(0)); // Black/Transparent
    }
    packets
}

pub fn decode_pvrtc_4bpp(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
    // PVRTC input data is packets.
    // 4bpp = 8 bytes per 4x4 block, aka 1 64-bit word per block.
    // Length check
    let expected_packets = (width * height / 16) as usize;
    if data.len() < expected_packets * 8 {
        return Err(RsbError::DeserializationError(
            "Insufficient data for PVRTC".into(),
        ));
    }

    let mut packets = Vec::with_capacity(expected_packets);
    for i in 0..expected_packets {
        let offset = i * 8;
        let word = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        packets.push(PvrTcPacket::new(word));
    }

    let pixels = decode_4bpp(&packets, width as i32);

    let mut img_buf = ImageBuffer::new(width, height);
    for (i, p) in pixels.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        if y < height {
            img_buf.put_pixel(x, y, p.to_pixel());
        }
    }

    Ok(DynamicImage::ImageRgba8(img_buf))
}

pub fn decode_pvrtc_4bpp_a8(
    data_pvrtc: &[u8],
    data_alpha: &[u8],
    width: u32,
    height: u32,
) -> Result<DynamicImage> {
    // 1. Decode PVRTC (RGB)
    let mut img = decode_pvrtc_4bpp(data_pvrtc, width, height)?;

    // 2. Decode Alpha (Uncompressed A8)
    if data_alpha.len() < (width * height) as usize {
        return Err(RsbError::DeserializationError(
            "Insufficient alpha data for PVRTC+A8".into(),
        ));
    }

    for y in 0..height {
        for x in 0..width {
            let mut p = img.get_pixel(x, y);
            let a = data_alpha[(y * width + x) as usize];
            p[3] = a;
            img.put_pixel(x, y, p);
        }
    }

    Ok(img)
}
