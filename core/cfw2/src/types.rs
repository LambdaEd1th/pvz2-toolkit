use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterFontWidget2 {
    pub ascent: i32,
    pub ascent_padding: i32,
    pub height: i32,
    pub line_spacing_offset: i32,
    pub initialized: bool,
    pub default_point_size: i32,
    pub character: Vec<CharacterItem>,
    pub layer: Vec<FontLayer>,
    pub source_file: String,
    pub error_header: String,
    pub point_size: i32,
    pub tag: Vec<String>,
    pub scale: f64,
    pub force_scaled_image_white: bool,
    pub activate_all_layer: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterItem {
    pub index: u16,
    pub value: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FontLayer {
    pub name: String,
    pub tag_require: Vec<String>,
    pub tag_exclude: Vec<String>,
    pub kerning: Vec<FontKerning>,
    pub character: Vec<FontCharacter>,
    pub multiply_red: i32,
    pub multiply_green: i32,
    pub multiply_blue: i32,
    pub multiply_alpha: i32,
    pub add_red: i32,
    pub add_green: i32,
    pub add_blue: i32,
    pub add_alpha: i32,
    pub image_file: String,
    pub draw_mode: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub spacing: i32,
    pub minimum_point_size: i32,
    pub maximum_point_size: i32,
    pub point_size: i32,
    pub ascent: i32,
    pub ascent_padding: i32,
    pub height: i32,
    pub default_height: i32,
    pub line_spacing_offset: i32,
    pub base_order: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FontCharacter {
    pub index: u16,
    pub image_rect_x: i32,
    pub image_rect_y: i32,
    pub image_rect_width: i32,
    pub image_rect_height: i32,
    pub image_offset_x: i32,
    pub image_offset_y: i32,
    pub kerning_count: u16,
    pub kerning_first: u16,
    pub width: i32,
    pub order: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FontKerning {
    pub offset: u16,
    pub index: u16,
}
