// ADAPTED FROM:
//
// (c) 2020 Tijn Porcelijn
//
// Simple MPEG2 Transport Stream demuxer
// - start with PAT handler listening to PID=0
// - from PAT create PMT listener to PID specified in PAT
// - from PMT create PES listeners for AAC or h264 elementary streams
//
// See: https://en.wikipedia.org/wiki/MPEG_transport_stream
//

use std::io::{self, ErrorKind};

use bytes::Bytes;

const PACKET_SIZE: usize = 188;

fn invalid(message: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message)
}

fn get_pid(packet: &[u8]) -> u16 {
    ((packet[1] as u16 & 0x1f) << 8) | packet[2] as u16
}

fn get_pusi(packet: &[u8]) -> bool {
    packet[1] & 0x40 != 0
}

fn get_payload(packet: &[u8]) -> io::Result<Option<&[u8]>> {
    if packet[0] != 0x47 {
        return Err(invalid("Invalid MPEG-TS sync byte"));
    }

    if packet[1] & 0x80 != 0 {
        return Err(invalid("MPEG-TS transport error"));
    }

    let control = (packet[3] >> 4) & 0x03;

    let offset = match control {
        0b01 => 4,
        0b10 => return Ok(None),
        0b11 => {
            let length = packet[4] as usize;
            let offset = 5 + length;

            if offset > PACKET_SIZE {
                return Err(invalid("Invalid adaptation field length"));
            }

            offset
        }
        _ => return Err(invalid("Invalid adaptation field control")),
    };

    if offset == PACKET_SIZE {
        return Ok(None);
    }

    Ok(Some(&packet[offset..]))
}

#[derive(Default)]
struct SectionReader {
    buffer: Vec<u8>,
}

impl SectionReader {
    fn reset(&mut self) {
        self.buffer.clear();
    }

    fn feed(&mut self, mut data: &[u8]) -> io::Result<Vec<Vec<u8>>> {
        let mut sections = Vec::new();

        while !data.is_empty() {
            if self.buffer.is_empty() && data[0] == 0xff {
                break;
            }

            if self.buffer.len() < 3 {
                let needed = 3 - self.buffer.len();
                let count = needed.min(data.len());

                self.buffer.extend_from_slice(&data[..count]);
                data = &data[count..];

                if self.buffer.len() < 3 {
                    break;
                }
            }

            let section_length = ((self.buffer[1] as usize & 0x0f) << 8) | self.buffer[2] as usize;

            if !(4..=1021).contains(&section_length) {
                return Err(invalid("Invalid PSI section length"));
            }

            let total = 3 + section_length;
            let needed = total - self.buffer.len();
            let count = needed.min(data.len());

            self.buffer.extend_from_slice(&data[..count]);
            data = &data[count..];

            if self.buffer.len() == total {
                sections.push(std::mem::take(&mut self.buffer));
            }
        }

        Ok(sections)
    }

    fn process(&mut self, payload: &[u8], pusi: bool) -> io::Result<Vec<Vec<u8>>> {
        if !pusi {
            if self.buffer.is_empty() {
                return Ok(Vec::new());
            }

            return self.feed(payload);
        }

        if payload.is_empty() {
            return Err(invalid("Missing PSI pointer field"));
        }

        let pointer = payload[0] as usize;
        let data = &payload[1..];

        if pointer > data.len() {
            return Err(invalid("Invalid PSI pointer field"));
        }

        let mut sections = Vec::new();

        if !self.buffer.is_empty() {
            sections.extend(self.feed(&data[..pointer])?);
        }

        self.reset();
        sections.extend(self.feed(&data[pointer..])?);

        Ok(sections)
    }
}

fn program_association_table(section: &[u8]) -> io::Result<Option<u16>> {
    if section.first() != Some(&0x00) {
        return Ok(None);
    }

    if section.len() < 12 {
        return Err(invalid("Invalid PAT section"));
    }

    let end = section.len() - 4;
    let mut offset = 8;

    while offset + 4 <= end {
        let program_number = u16::from_be_bytes([section[offset], section[offset + 1]]);

        let pid = ((section[offset + 2] as u16 & 0x1f) << 8) | section[offset + 3] as u16;

        if program_number != 0 {
            return Ok(Some(pid));
        }

        offset += 4;
    }

    Ok(None)
}

fn program_map_table(section: &[u8]) -> io::Result<Option<u16>> {
    if section.first() != Some(&0x02) {
        return Ok(None);
    }

    if section.len() < 16 {
        return Err(invalid("Invalid PMT section"));
    }

    let end = section.len() - 4;

    let program_info_length = ((section[10] as usize & 0x0f) << 8) | section[11] as usize;

    let mut offset = 12 + program_info_length;

    if offset > end {
        return Err(invalid("Invalid PMT program info length"));
    }

    while offset < end {
        if offset + 5 > end {
            return Err(invalid("Incomplete PMT stream entry"));
        }

        let stream_type = section[offset];

        let pid = ((section[offset + 1] as u16 & 0x1f) << 8) | section[offset + 2] as u16;

        let es_info_length =
            ((section[offset + 3] as usize & 0x0f) << 8) | section[offset + 4] as usize;

        offset += 5 + es_info_length;

        if offset > end {
            return Err(invalid("Invalid PMT elementary stream length"));
        }

        if stream_type == 0x0f {
            return Ok(Some(pid));
        }
    }

    Ok(None)
}

#[derive(Default)]
struct PesReader {
    buffer: Vec<u8>,
}

impl PesReader {
    fn process(&mut self, payload: &[u8], pusi: bool, output: &mut Vec<u8>) -> io::Result<()> {
        if pusi {
            self.flush(output)?;
        } else if self.buffer.is_empty() {
            return Ok(());
        }

        self.buffer.extend_from_slice(payload);

        Ok(())
    }

    fn flush(&mut self, output: &mut Vec<u8>) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let pes = std::mem::take(&mut self.buffer);

        if pes.len() < 9 {
            return Err(invalid("Incomplete PES header"));
        }

        if pes[..3] != [0x00, 0x00, 0x01] {
            return Err(invalid("Invalid PES start code"));
        }

        let header_size = 9 + pes[8] as usize;

        if header_size > pes.len() {
            return Err(invalid("Invalid PES header length"));
        }

        let packet_length = u16::from_be_bytes([pes[4], pes[5]]) as usize;

        let end = if packet_length == 0 {
            pes.len()
        } else {
            (6 + packet_length).min(pes.len())
        };

        if end < header_size {
            return Err(invalid("Invalid PES packet length"));
        }

        output.extend_from_slice(&pes[header_size..end]);

        Ok(())
    }
}

pub async fn ts_demux(segment: Bytes) -> io::Result<Bytes> {
    if segment.len() % PACKET_SIZE != 0 {
        return Err(invalid("Incomplete MPEG-TS packet"));
    }

    let mut pat_reader = SectionReader::default();
    let mut pmt_reader = SectionReader::default();
    let mut pes_reader = PesReader::default();

    let mut pmt_pid = None;
    let mut audio_pid = None;

    let mut output = Vec::new();

    for packet in segment.chunks_exact(PACKET_SIZE) {
        if packet[0] != 0x47 {
            return Err(invalid("Invalid MPEG-TS sync byte"));
        }

        let pid = get_pid(packet);

        if pid != 0 && Some(pid) != pmt_pid && Some(pid) != audio_pid {
            continue;
        }

        let Some(payload) = get_payload(packet)? else {
            continue;
        };

        let pusi = get_pusi(packet);

        if pid == 0 {
            for section in pat_reader.process(payload, pusi)? {
                if let Some(pid) = program_association_table(&section)? {
                    if pmt_pid != Some(pid) {
                        pmt_reader.reset();
                        pmt_pid = Some(pid);
                    }
                }
            }
        } else if Some(pid) == pmt_pid {
            for section in pmt_reader.process(payload, pusi)? {
                if let Some(pid) = program_map_table(&section)? {
                    audio_pid = Some(pid);
                }
            }
        } else if Some(pid) == audio_pid {
            pes_reader.process(payload, pusi, &mut output)?;
        }
    }

    pes_reader.flush(&mut output)?;

    if audio_pid.is_none() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "No ADTS AAC stream found in MPEG-TS segment",
        ));
    }

    if output.is_empty() {
        return Err(invalid("No AAC audio data found in MPEG-TS segment"));
    }

    Ok(Bytes::from(output))
}
