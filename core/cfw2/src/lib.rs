pub mod error;
pub mod reader;
pub mod types;
pub mod writer;

pub use error::{Cfw2Error, Result};
pub use reader::*;
pub use types::*;
pub use writer::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn make_sample() -> CharacterFontWidget2 {
        CharacterFontWidget2 {
            ascent: 20,
            ascent_padding: 2,
            height: 32,
            line_spacing_offset: 1,
            initialized: true,
            default_point_size: 12,
            character: vec![
                CharacterItem {
                    index: 65,
                    value: 66,
                },
                CharacterItem {
                    index: 67,
                    value: 68,
                },
            ],
            layer: vec![FontLayer {
                name: "default".to_string(),
                tag_require: vec!["en".to_string()],
                tag_exclude: vec![],
                kerning: vec![FontKerning {
                    offset: 1,
                    index: 65,
                }],
                character: vec![FontCharacter {
                    index: 65,
                    image_rect_x: 0,
                    image_rect_y: 0,
                    image_rect_width: 16,
                    image_rect_height: 32,
                    image_offset_x: 0,
                    image_offset_y: 0,
                    kerning_count: 1,
                    kerning_first: 0,
                    width: 16,
                    order: 0,
                }],
                multiply_red: 255,
                multiply_green: 255,
                multiply_blue: 255,
                multiply_alpha: 255,
                add_red: 0,
                add_green: 0,
                add_blue: 0,
                add_alpha: 0,
                image_file: "font_default.png".to_string(),
                draw_mode: 0,
                offset_x: 0,
                offset_y: 0,
                spacing: 0,
                minimum_point_size: 8,
                maximum_point_size: 72,
                point_size: 12,
                ascent: 20,
                ascent_padding: 2,
                height: 32,
                default_height: 32,
                line_spacing_offset: 1,
                base_order: 0,
            }],
            source_file: "font.txt".to_string(),
            error_header: "".to_string(),
            point_size: 12,
            tag: vec!["main".to_string()],
            scale: 1.0,
            force_scaled_image_white: false,
            activate_all_layer: true,
        }
    }

    #[test]
    fn test_cfw2_roundtrip() {
        let original = make_sample();

        let mut buf = Vec::new();
        encode(&mut buf, &original).expect("encode failed");

        let mut cursor = Cursor::new(buf);
        let decoded = decode(&mut cursor).expect("decode failed");

        assert_eq!(decoded.ascent, original.ascent);
        assert_eq!(decoded.height, original.height);
        assert_eq!(decoded.initialized, original.initialized);
        assert_eq!(decoded.character.len(), original.character.len());
        assert_eq!(decoded.character[0].index, 65);
        assert_eq!(decoded.character[0].value, 66);
        assert_eq!(decoded.layer.len(), 1);
        assert_eq!(decoded.layer[0].name, "default");
        assert_eq!(decoded.layer[0].character.len(), 1);
        assert_eq!(decoded.layer[0].character[0].image_rect_width, 16);
        assert_eq!(decoded.layer[0].image_file, "font_default.png");
        assert_eq!(decoded.source_file, "font.txt");
        assert_eq!(decoded.tag, vec!["main"]);
        assert_eq!(decoded.scale, 1.0);
        assert_eq!(decoded.activate_all_layer, true);
    }
}
