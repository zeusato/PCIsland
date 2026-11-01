//! Continuous Opus encoder with a durable packet journal. Clips remux packets
//! with 80 ms decoder preroll and correct pre-skip/end granule, without re-encoding.
#![forbid(unsafe_code)]
use ogg::{PacketWriteEndInfo, PacketWriter};
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    path::Path,
};
pub const FRAME: usize = 960;
pub const RATE: u64 = 48000;
const SERIAL: u32 = 0x50434953;
pub struct Recorder {
    encoder: opus::Encoder,
    journal: BufWriter<File>,
    pub frames: u64,
}
impl Recorder {
    pub fn new(path: &Path) -> Result<Self, String> {
        let mut encoder =
            opus::Encoder::new(RATE as u32, opus::Channels::Mono, opus::Application::Audio)
                .map_err(|e| e.to_string())?;
        encoder
            .set_bitrate(opus::Bitrate::Bits(48000))
            .map_err(|e| e.to_string())?;
        let preskip = encoder.get_lookahead().map_err(|e| e.to_string())? as u16;
        let mut journal = BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
        journal
            .write_all(&preskip.to_le_bytes())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            encoder,
            journal,
            frames: 0,
        })
    }
    pub fn push(&mut self, frame: &[f32]) -> Result<(), String> {
        let mut output = [0_u8; 4096];
        let size = self
            .encoder
            .encode_float(frame, &mut output)
            .map_err(|e| e.to_string())?;
        self.journal
            .write_all(&(size as u16).to_le_bytes())
            .and_then(|()| self.journal.write_all(&output[..size]))
            .map_err(|e| e.to_string())?;
        self.frames += 1;
        Ok(())
    }
    pub fn flush(&mut self) -> Result<(), String> {
        self.journal
            .flush()
            .and_then(|()| self.journal.get_ref().sync_data())
            .map_err(|e| e.to_string())
    }
}
fn headers(writer: &mut PacketWriter<'_, File>, preskip: u16) -> Result<(), String> {
    let mut head = b"OpusHead\x01\x01".to_vec();
    head.extend(preskip.to_le_bytes());
    head.extend((RATE as u32).to_le_bytes());
    head.extend([0, 0, 0]);
    writer
        .write_packet(head, SERIAL, PacketWriteEndInfo::EndPage, 0)
        .map_err(|e| e.to_string())?;
    let vendor = b"PCisland";
    let mut tags = b"OpusTags".to_vec();
    tags.extend((vendor.len() as u32).to_le_bytes());
    tags.extend(vendor);
    tags.extend(0_u32.to_le_bytes());
    writer
        .write_packet(tags, SERIAL, PacketWriteEndInfo::EndPage, 0)
        .map_err(|e| e.to_string())
}
pub fn export(journal: &Path, dest: &Path, start_ms: u64, end_ms: u64) -> Result<(), String> {
    let mut input = File::open(journal).map_err(|e| e.to_string())?;
    let mut short = [0; 2];
    input.read_exact(&mut short).map_err(|e| e.to_string())?;
    let lookahead = u64::from(u16::from_le_bytes(short));
    let start = start_ms * 48;
    let end = end_ms * 48;
    if end <= start {
        return Err("Empty clip".into());
    }
    let first = (start / (FRAME as u64)).saturating_sub(4);
    let skip = start - first * FRAME as u64 + lookahead;
    let last = (end + lookahead).div_ceil(FRAME as u64);
    let temp = dest.with_extension(format!("{}.opus.partial", crate::core::id::new_id()));
    let result = (|| {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let mut writer = PacketWriter::new(file);
        headers(&mut writer, skip as u16)?;
        for index in 0..last {
            input.read_exact(&mut short).map_err(|e| e.to_string())?;
            let size = u16::from_le_bytes(short) as usize;
            if size == 0 || size > 4096 {
                return Err("Invalid audio journal".into());
            }
            let mut packet = vec![0; size];
            input.read_exact(&mut packet).map_err(|e| e.to_string())?;
            if index < first {
                continue;
            }
            let final_packet = index + 1 == last;
            let gp = if final_packet {
                skip + end - start
            } else {
                (index - first + 1) * FRAME as u64
            };
            writer
                .write_packet(
                    packet,
                    SERIAL,
                    if final_packet {
                        PacketWriteEndInfo::EndStream
                    } else if index % 250 == 249 {
                        PacketWriteEndInfo::EndPage
                    } else {
                        PacketWriteEndInfo::NormalPacket
                    },
                    gp,
                )
                .map_err(|e| e.to_string())?;
        }
        writer.into_inner().sync_all().map_err(|e| e.to_string())?;
        crate::platform::files::publish(&temp, dest).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = std::fs::remove_file(temp);
    result
}
pub fn recover_duration(path: &Path) -> Result<u64, String> {
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let mut short = [0; 2];
    input.read_exact(&mut short).map_err(|e| e.to_string())?;
    let skip = u16::from_le_bytes(short) as u64;
    let mut count = 0;
    while input.read_exact(&mut short).is_ok() {
        let size = u16::from_le_bytes(short) as usize;
        if size == 0 || size > 4096 {
            break;
        }
        let mut data = vec![0; size];
        if input.read_exact(&mut data).is_err() {
            break;
        }
        count += 1;
    }
    Ok((count * FRAME as u64).saturating_sub(skip) / 48)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_and_preroll_clip_are_decodable() {
        let dir = std::env::temp_dir().join(crate::core::id::new_id());
        std::fs::create_dir(&dir).unwrap();
        let journal = dir.join("audio.packets");
        let mut r = Recorder::new(&journal).unwrap();
        for n in 0..251 {
            let frame: Vec<f32> = (0..FRAME)
                .map(|i| ((n * FRAME + i) as f32 * 0.02).sin() * 0.1)
                .collect();
            r.push(&frame).unwrap();
        }
        r.flush().unwrap();
        drop(r);
        export(&journal, &dir.join("full.opus"), 0, 5000).unwrap();
        export(&journal, &dir.join("clip.opus"), 1500, 3500).unwrap();
        let mut reader = ogg::PacketReader::new(File::open(dir.join("clip.opus")).unwrap());
        let head = reader.read_packet().unwrap().unwrap();
        let skip = u16::from_le_bytes([head.data[10], head.data[11]]) as u64;
        reader.read_packet().unwrap();
        let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();
        let mut pcm = [0.0; 5760];
        let mut last = 0;
        while let Some(p) = reader.read_packet().unwrap() {
            assert!(decoder.decode_float(&p.data, &mut pcm, false).unwrap() > 0);
            last = p.absgp_page();
        }
        assert_eq!(last - skip, 96000);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
