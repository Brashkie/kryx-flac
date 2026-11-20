//! FLAC frame CRCs, computed bytewise (no lookup tables, no deps).
//!
//! - CRC-8  over the frame header: polynomial `0x07`, init `0x00`.
//! - CRC-16 over the whole frame:  polynomial `0x8005`, init `0x0000`.
//!
//! Both are MSB-first with no input/output reflection, matching the FLAC spec.

/// CRC-8 (poly `0x07`, init `0`) of `data`.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// CRC-16 (poly `0x8005`, init `0`) of `data`.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc8_known_vectors() {
        assert_eq!(crc8(&[]), 0x00);
        // "123456789" is the standard CRC check string.
        assert_eq!(crc8(b"123456789"), 0xF4);
    }

    #[test]
    fn crc16_known_vectors() {
        assert_eq!(crc16(&[]), 0x0000);
        // CRC-16/UMTS (aka BUYPASS): poly 0x8005, init 0, check = 0xFEE8.
        assert_eq!(crc16(b"123456789"), 0xFEE8);
    }
}
