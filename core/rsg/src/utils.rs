use crate::error::Result;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};

pub trait FileListPayload: Sized {
    fn read(reader: &mut impl Read) -> Result<Self>;
    fn write(&self, writer: &mut impl Write) -> Result<()>;
}

impl FileListPayload for i32 {
    fn read(reader: &mut impl Read) -> Result<Self> {
        Ok(reader.read_i32::<LE>()?)
    }

    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_i32::<LE>(*self)?;
        Ok(())
    }
}

pub fn read_file_list<P: FileListPayload, R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    length: u64,
) -> Result<Vec<(String, P)>> {
    reader.seek(SeekFrom::Start(start_offset))?;
    let limit = start_offset + length;

    let mut file_list = Vec::new();
    let mut name_dict: HashMap<u64, String> = HashMap::new();
    let mut current_path = String::new();

    while reader.stream_position()? < limit {
        let char_byte = reader.read_u8()?;
        let offset_val = reader.read_u24::<LE>()? as u64 * 4;

        // The offset points to a "Sibling" node which shares the same prefix as the current node.
        // Therefore, we must register the *current* path (before appending char) for that offset.
        if offset_val != 0 {
            name_dict.insert(offset_val + start_offset, current_path.clone());
        }

        if char_byte == 0 {
            let payload = P::read(reader)?;
            file_list.push((current_path.clone(), payload));

            // Logic check: When we finish a file (char=0), we might "jump" if this node was a target.
            // But wait, "char=0" is a node. Does it have children?
            // In this stream format, children follow immediately.
            // If char=0, it's a leaf efficiently.
            // But checking `name_dict` here is correct because the NEXT node in stream ensures checking.
            // Wait, the `name_dict` check is for the START of a node loop.
            // Here we check it *after* processing?

            // The loop reads [Char][Offset].
            // If we just finished [0][Offset], the NEXT byte is the start of the next node.
            // That node might be a jump target.

            let current_pos = reader.stream_position()?;
            if let Some(path) = name_dict.remove(&current_pos) {
                current_path = path;
            }
        } else {
            current_path.push(char_byte as char);

            // Note: We ALREADY handled insertion for offset_val above using the prefix.
        }
    }

    Ok(file_list)
}

pub fn write_file_list<P: FileListPayload, W: Write + Seek>(
    writer: &mut W,
    start_offset: u64,
    items: &[(String, P)],
) -> Result<()> {
    // Robust Packer with Prefix Compression

    // 1. Prepare items (dummy empty item handled by caller or we inject?)
    // We will inject a dummy "" item if not present, because the compression logic relies on it as a root.
    // However, if we inject it, we must provide a dummy Payload?
    // We can't fabricate P.
    // So we invoke P::default()? No constraint.
    // We assume the Caller has provided the dummy item if they want optimal compression.
    // If not, we start with the first item fully.

    let mut sorted_items: Vec<(String, P)> = items
        .iter()
        .map(|(s, p)| (s.clone(), unsafe { std::ptr::read(p) }))
        .collect();

    sorted_items.sort_by(|a, b| a.0.to_uppercase().cmp(&b.0.to_uppercase()));

    // Track active prefix states: (Path, FilePositionOfOffsetField)
    // The FilePositionOfOffsetField is where we wrote the 0 that we need to update later.
    let mut active_prefixes: HashMap<String, u64> = HashMap::new();

    // We also need to track the "current path" state of the artificial parser
    let mut current_parser_path = String::new();

    for (path, payload) in sorted_items {
        // Compute LCP between `current_parser_path` and `path`
        // But wait, the parser state is determined by where we JUMP to.
        // We find the longest prefix in `active_prefixes` that matches `path`.

        // let mut best_prefix = String::new(); // Removed unused variable
        // Since we iterate sorted, the best prefix is likely from the immediate previous item or its ancestors.
        // We can search `active_prefixes` for the longest match.
        // Optimization: Check prefixes of `path` from longest to shortest?

        // Find split point
        let mut split_idx = 0;
        let char_indices: Vec<(usize, char)> = path.char_indices().collect();

        for i in (0..=char_indices.len()).rev() {
            let prefix = if i == char_indices.len() {
                path.clone()
            } else {
                path[..char_indices[i].0].to_string()
            };

            if let Some(&instr_pos) = active_prefixes.get(&prefix) {
                // Found a reusable state!
                // Update the instruction at `instr_pos` to point to HERE.
                // Calculate Target Offset: (CurrentPos - start_offset) / 4.

                let current_pos = writer.stream_position()?;
                let target_val = (current_pos - start_offset) / 4;

                // Update the previous jump instruction
                let saved_pos = writer.stream_position()?;
                writer.seek(SeekFrom::Start(instr_pos))?;
                writer.write_u24::<LE>(target_val as u32)?; // Update the offset
                writer.seek(SeekFrom::Start(saved_pos))?;

                // Remove this prefix from active because we "consumed" the jump?
                // No, multiple items might jump here?
                // C# implementation: `pathTempList.Remove`?
                // `check nameDict.RemoveAt(i)` in reader implies one-time use?
                // Yes, `nameDict.RemoveAt(i)`.
                // So a jump point is good for ONE reuse.
                // Remove from active_prefixes.
                active_prefixes.remove(&prefix);

                current_parser_path = prefix;
                if i < char_indices.len() {
                    split_idx = i;
                } else {
                    split_idx = char_indices.len();
                }
                break;
            }
        }

        // Write Suffix
        // Suffix starts after `best_prefix`.
        // If best_prefix is empty, we write full string.

        // If best_prefix was found, split_idx matches its length char count.
        // If not found, split_idx = 0.

        let suffix_chars = &char_indices[split_idx..];

        for (_, c) in suffix_chars.iter() {
            // Write Char
            writer.write_u8(*c as u8)?;

            // Write Placeholder Offset
            let offset_pos = writer.stream_position()?;
            writer.write_u24::<LE>(0)?;

            // Record this new state as available for future jumps
            // State: current_parser_path + char c (and previous chars)
            current_parser_path.push(*c);

            // We only need to store this state if it's NOT the last char (which is \0 termin)
            // Actually C# `writeStringFourByte` assumes chars.
            // But we treat the string construction char by char.

            active_prefixes.insert(current_parser_path.clone(), offset_pos);
        }

        // Finish string with \0
        writer.write_u8(0)?;
        let offset_pos = writer.stream_position()?;
        writer.write_u24::<LE>(0)?;

        // State after \0 is valid for next item if it matches full string?
        // Reader: if char==0, we insert `current_path` to dict if offset!=0.
        // So yes, we can jump to the state "After String Finished".
        // But `current_parser_path` doesn't include \0.
        active_prefixes.insert(current_parser_path.clone(), offset_pos);

        // Write Payload
        payload.write(writer)?;

        // `current_parser_path` remains set.
        // But next iteration starts by looking for jump.
    }

    Ok(())
}
