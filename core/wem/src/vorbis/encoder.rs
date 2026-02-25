use crate::error::{WemError, WemResult};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ogg::{Packet, PacketReader};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

/// Converts a standard Ogg Vorbis file to a Wwise RIFF/WEM file (Standard Header).
///
/// This implementation uses the "Standard Header" format which includes the full
/// setup header (codebooks, etc.) embedded in the file. This is the most compatible
/// format and avoids the need for external codebook matching/stripping.
pub struct OggToWem<R: Read + Seek> {
    input: PacketReader<R>,
    // output: Cursor<Vec<u8>>, // Unused
    channels: u16,
    sample_rate: u32,
    avg_bytes_per_second: u32,
    blocksize_0: u8,
    blocksize_1: u8,
    total_samples: u32,
    setup_packet_offset: u32,
    first_audio_packet_offset: u32,
    data_size: u32,
}

impl<R: Read + Seek> OggToWem<R> {
    pub fn new(input: R) -> Self {
        Self {
            input: PacketReader::new(input),
            channels: 0,
            sample_rate: 0,
            avg_bytes_per_second: 0,
            blocksize_0: 0,
            blocksize_1: 0,
            total_samples: 0,
            setup_packet_offset: 0,
            first_audio_packet_offset: 0,
            data_size: 0,
        }
    }

    pub fn process<W: Write>(&mut self, mut writer: W) -> WemResult<()> {
        // We will process packets, gather info, and build the 'data' chunk content.
        let mut vorbis_packets = Vec::new();

        while let Some(p) = self
            .input
            .read_packet()
            .map_err(|e| WemError::parse(format!("Ogg error: {:?}", e)))?
        {
            vorbis_packets.push(p);
        }

        if vorbis_packets.len() < 3 {
            return Err(WemError::parse(
                "Input OGG has too few packets (missing headers?)",
            ));
        }

        // 1. Parse Headers
        self.parse_id_header(&vorbis_packets[0].data)?;
        // Comment header (packets[1]) is ignored/stripped
        // Setup header (packets[2]) is kept full

        // 2. Prepare Data Chunk
        // The data chunk in WEM (Standard Header) contains:
        // - Setup Packet (with size/granule header)
        // - Audio Packets (with size/granule header)

        // We need to write them into a temporary buffer to calculate data_size and offsets
        let mut data_chunk_body = Vec::new();
        let mut data_cursor = Cursor::new(&mut data_chunk_body);

        // Write Setup Packet
        self.setup_packet_offset = 0; // Relative to data body start

        // Strip Ogit header (0x05 + "vorbis") from setup packet
        let setup_data = &vorbis_packets[2].data;
        let setup_payload =
            if setup_data.len() > 7 && setup_data[0] == 5 && &setup_data[1..7] == b"vorbis" {
                &setup_data[7..]
            } else {
                setup_data
            };

        // Custom write for setup packet to avoid cloning Packet just to strip data
        let size = setup_payload.len() as u32;
        data_cursor.write_u16::<LittleEndian>(size as u16)?;
        data_cursor.write_u32::<LittleEndian>(0)?; // Granule 0 for setup
        data_cursor.write_all(setup_payload)?;

        // Write Audio Packets
        self.first_audio_packet_offset = data_cursor.position() as u32;

        let mut last_granule = 0;
        for p in vorbis_packets.iter().skip(3) {
            self.write_packet(&mut data_cursor, p, false)?;
            last_granule = p.absgp_page();
        }

        self.total_samples = last_granule as u32;
        self.data_size = data_cursor.position() as u32;

        // Calculate avg bytes per second (approx)
        if self.total_samples > 0 {
            let duration = self.total_samples as f64 / self.sample_rate as f64;
            if duration > 0.0 {
                self.avg_bytes_per_second = (self.data_size as f64 / duration) as u32;
            }
        }

        // 3. Write RIFF Structure to output
        // RIFF Header
        writer.write_all(b"RIFF")?;
        // File size - 8. We need to calculate total size.
        // RIFF(12) + fmt(0x42+8) + vorb(0x28+or+more+8) + data(header+size) + smpl(optional)

        // fmt chunk size: 0x18 (24 bytes) allows separate vorb chunk
        let fmt_chunk_size = 0x18;
        // vorb chunk size: 0x34 (52 bytes) for Standard Header / Modern
        let vorb_chunk_size = 0x34;
        // data chunk header: 8 bytes

        let riff_payload_size = 4 + // WAVE
                                8 + fmt_chunk_size + // fmt
                                8 + vorb_chunk_size + // vorb
                                8 + self.data_size; // data

        writer.write_u32::<LittleEndian>(riff_payload_size)?;
        writer.write_all(b"WAVE")?;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_u32::<LittleEndian>(fmt_chunk_size)?;
        writer.write_u16::<LittleEndian>(0xFFFF)?; // Codec ID (Vorbis)
        writer.write_u16::<LittleEndian>(self.channels)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;
        writer.write_u32::<LittleEndian>(self.avg_bytes_per_second)?;
        writer.write_u16::<LittleEndian>(0)?; // Block Align
        writer.write_u16::<LittleEndian>(0)?; // Bits per sample
        writer.write_u16::<LittleEndian>(6)?; // Extra size (24 - 18)
        writer.write_u16::<LittleEndian>(0)?; // ext_unk
        writer.write_u32::<LittleEndian>(3)?; // subtype (3=Standard Header)

        // vorb chunk
        writer.write_all(b"vorb")?;
        writer.write_u32::<LittleEndian>(vorb_chunk_size)?;
        writer.write_u32::<LittleEndian>(self.total_samples)?;

        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;

        writer.write_u32::<LittleEndian>(self.setup_packet_offset)?;
        writer.write_u32::<LittleEndian>(self.first_audio_packet_offset)?;

        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;

        writer.write_u32::<LittleEndian>(0xABCDEF01)?; // UID (Random)
        writer.write_u8(self.blocksize_0)?;
        writer.write_u8(self.blocksize_1)?;
        writer.write_u16::<LittleEndian>(0)?;

        // data chunk
        writer.write_all(b"data")?;
        writer.write_u32::<LittleEndian>(self.data_size)?;

        // Write data content
        writer.write_all(&data_chunk_body)?;

        Ok(())
    }

    fn parse_id_header(&mut self, data: &[u8]) -> WemResult<()> {
        let mut r = Cursor::new(data);
        // packet_type (1) + "vorbis" (6)
        r.seek(SeekFrom::Start(7))?;

        // version (4)
        let _ver = r.read_u32::<LittleEndian>()?;
        self.channels = r.read_u8()? as u16;
        self.sample_rate = r.read_u32::<LittleEndian>()?;
        let _bitrate_max = r.read_i32::<LittleEndian>()?;
        let _bitrate_nom = r.read_i32::<LittleEndian>()?;
        let _bitrate_min = r.read_i32::<LittleEndian>()?;
        let blocksizes = r.read_u8()?;

        self.blocksize_0 = blocksizes & 0x0F;
        self.blocksize_1 = (blocksizes >> 4) & 0x0F;

        Ok(())
    }

    fn write_packet<W: Write>(&self, w: &mut W, p: &Packet, no_granule: bool) -> WemResult<()> {
        let size = p.data.len() as u32;

        // 6-byte header: size (2) + granule (4)
        // OR 2-byte header: size (2)
        // Standard header usually uses 6-byte for audio packets.
        // Setup packet might be different?
        // Since we chose vorb 0x34 -> mod_packets=false -> `no_granule` defaults to false in reader (unless 0x2A).
        // So we should write 6 byte header.

        w.write_u16::<LittleEndian>(size as u16)?;

        if !no_granule {
            // Granule is 4 bytes.
            // OGG packet has u64 granule_pos. WEM uses u32?
            // Packet struct has `granule: u32`.
            let g = p.absgp_page();
            w.write_u32::<LittleEndian>(g as u32)?;
        }

        w.write_all(&p.data)?;
        Ok(())
    }
}
