use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PopcapRenderEffectObject {
    pub block_1: Vec<Block1>,
    pub block_2: Vec<Block2>,
    pub block_3: Vec<Block3>,
    pub block_4: Vec<Block4>,
    pub block_5: Vec<Block5>,
    pub block_6: Vec<Block6>,
    pub block_7: Vec<Block7>,
    pub block_8: Vec<Block8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block1 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
    pub unknown_6: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block2 {
    pub unknown_1: u32,
    pub unknown_2: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block3 {
    pub unknown_2: u32,
    pub string: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block4 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block5 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
    pub unknown_6: u32,
    pub unknown_7: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block6 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block7 {
    pub unknown_1: u32,
    pub unknown_2: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block8 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

pub const BLOCK_SIZES: [u32; 8] = [0x18, 0x08, 0x0C, 0x14, 0x1C, 0x14, 0x08, 0x14];
