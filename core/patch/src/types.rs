pub struct RsbPatchHeader {
    pub rsb_after_size: i32,
    pub rsb_head_size: i32,
    pub md5_before: [u8; 16],
    pub rsg_number: i32,
    pub rsb_need_patch: bool,
}

pub struct RsbPatchPacketInfo {
    pub packet_patch_size: i32,
    pub packet_name: String,
    pub md5_packet: [u8; 16],
}
