pub mod codec;
pub mod color;
pub mod decoder;
pub mod encoder;
pub mod types;

pub use decoder::PtxDecoder;
pub use encoder::PtxEncoder;
pub use types::PtxFormat;

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba};

    #[test]
    fn test_process_wrappers() {
        // Simple test to verify wrappers work
        let width = 64;
        let height = 64;
        let img = DynamicImage::new_rgba8(width, height);
        let encoded = PtxEncoder::encode(&img, PtxFormat::Rgba8888, false).unwrap();
        assert_eq!(encoded.len(), (width * height * 4) as usize);

        // Decode check
        let decoded = PtxDecoder::decode(&encoded, width, height, 0, None, None, false).unwrap();
        assert_eq!(decoded.width(), width);
    }

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
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba8888, false).unwrap();
        assert_eq!(data.len(), (width * height * 4) as usize);

        // Rgba4444: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba4444, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgb565: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgb565, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba5551: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba5551, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba4444Block: 2 bytes per pixel (no padding needed for 64x64)
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba4444Block, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgb565Block: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgb565Block, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Rgba5551Block: 2 bytes per pixel
        let data = PtxEncoder::encode(&img, PtxFormat::Rgba5551Block, false).unwrap();
        assert_eq!(data.len(), (width * height * 2) as usize);

        // Etc1: 0.5 bytes per pixel (4x4 block = 8 bytes)
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1, false).unwrap();
        assert_eq!(data.len(), (width * height / 2) as usize);

        // Etc1A8: Etc1 + 1 byte per pixel alpha
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1A8, false).unwrap();
        assert_eq!(data.len(), (width * height / 2 + width * height) as usize);

        // Etc1Palette: Etc1 + 1 header + 16 palette + 0.5 bytes per pixel alpha
        let data = PtxEncoder::encode(&img, PtxFormat::Etc1Palette, false).unwrap();
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
        for item in data_a8.iter_mut().skip(opaque_size as usize) {
            *item = 255;
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
