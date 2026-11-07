use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgrtError {
    WrongLength,
    WrongSignature,
    TooShort,
}

impl core::fmt::Display for BgrtError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WrongLength => f.write_str("wrong BGRT table length"),
            Self::WrongSignature => f.write_str("wrong BGRT signature"),
            Self::TooShort => f.write_str("BGRT table too short"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgrtTable {
    pub version: u16,
    pub status: u8,
    pub image_type: u8,
    pub image_address: u64,
    pub image_offset_x: u32,
    pub image_offset_y: u32,
    pub checksum_ok: bool,
}

pub const BGRT_LEN: usize = 56;

pub fn center_offsets(image_w: u32, image_h: u32, screen_w: u32, screen_h: u32) -> (u32, u32) {
    let x = screen_w.saturating_sub(image_w) / 2;
    let y = screen_h.saturating_sub(image_h) / 2;
    (x, y)
}

pub fn build_bgrt(
    image_address: u64,
    image_w: u32,
    image_h: u32,
    screen_w: u32,
    screen_h: u32,
) -> Vec<u8> {
    let (ox, oy) = center_offsets(image_w, image_h, screen_w, screen_h);
    let mut table = alloc::vec![0u8; BGRT_LEN];
    table[0..4].copy_from_slice(b"BGRT");
    table[4..8].copy_from_slice(&(BGRT_LEN as u32).to_le_bytes());
    table[8] = 1;
    table[36..38].copy_from_slice(&1u16.to_le_bytes());
    table[38] = 1;
    table[39] = 0;
    table[40..48].copy_from_slice(&image_address.to_le_bytes());
    table[48..52].copy_from_slice(&ox.to_le_bytes());
    table[52..56].copy_from_slice(&oy.to_le_bytes());

    let mut sum: u8 = 0;
    for (i, b) in table.iter().enumerate() {
        if i == 9 {
            continue;
        }
        sum = sum.wrapping_add(*b);
    }
    table[9] = sum.wrapping_neg();
    table
}

impl BgrtTable {
    pub fn parse(bytes: &[u8]) -> Result<Self, BgrtError> {
        if bytes.len() < BGRT_LEN {
            return Err(BgrtError::TooShort);
        }
        if bytes.len() != BGRT_LEN {
            return Err(BgrtError::WrongLength);
        }
        if &bytes[0..4] != b"BGRT" {
            return Err(BgrtError::WrongSignature);
        }
        let version = u16::from_le_bytes([bytes[36], bytes[37]]);
        let status = bytes[38];
        let image_type = bytes[39];
        let image_address = u64::from_le_bytes([
            bytes[40], bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46], bytes[47],
        ]);
        let image_offset_x = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]);
        let image_offset_y = u32::from_le_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]);
        let checksum_ok = bytes.iter().fold(0u8, |a, b| a.wrapping_add(*b)) == 0;
        Ok(Self {
            version,
            status,
            image_type,
            image_address,
            image_offset_x,
            image_offset_y,
            checksum_ok,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_sums_to_zero() {
        let t = build_bgrt(0x1000, 100, 50, 1920, 1080);
        let sum: u8 = t.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        assert_eq!(sum, 0);
    }

    #[test]
    fn offsets_center() {
        assert_eq!(center_offsets(100, 50, 1920, 1080), (910, 515));
        assert_eq!(center_offsets(64, 48, 64, 48), (0, 0));
        assert_eq!(center_offsets(200, 100, 100, 50), (0, 0));
    }

    #[test]
    fn round_trip() {
        let t = build_bgrt(0xabcd, 64, 48, 320, 200);
        let p = BgrtTable::parse(&t).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.status, 1);
        assert_eq!(p.image_type, 0);
        assert_eq!(p.image_address, 0xabcd);
        assert!(p.checksum_ok);
        assert_eq!(p.image_offset_x, 128);
        assert_eq!(p.image_offset_y, 76);
    }

    #[test]
    fn refuse_wrong_length() {
        let t = build_bgrt(1, 4, 4, 8, 8);
        let mut short = t.clone();
        short.pop();
        assert!(matches!(
            BgrtTable::parse(&short),
            Err(BgrtError::TooShort) | Err(BgrtError::WrongLength)
        ));
        let mut long = t.clone();
        long.push(0);
        assert!(matches!(
            BgrtTable::parse(&long),
            Err(BgrtError::WrongLength)
        ));
    }

    #[test]
    fn refuse_wrong_signature() {
        let mut t = build_bgrt(1, 4, 4, 8, 8);
        t[0] = b'X';
        assert_eq!(BgrtTable::parse(&t), Err(BgrtError::WrongSignature));
    }
}
