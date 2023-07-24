use super::{Trb, TrbType};

pub struct NormalTrb {
    addr: u64,
    length: u32,
    td_size: u8,
}

impl NormalTrb {
    pub fn new(addr: u64, length: u32, td_size: u8) -> Self {
        Self {
            addr,
            length,
            td_size,
        }
    }
}

impl From<NormalTrb> for Trb {
    fn from(trb: NormalTrb) -> Self {
        let status = trb.length & 0x1FFFF | (trb.td_size as u32) << 17;
        Self {
            parameter: trb.addr,
            status,
            control: TrbType::Normal.to_control(),
        }
    }
}