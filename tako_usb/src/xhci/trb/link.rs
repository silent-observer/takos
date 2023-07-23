use super::{Trb, TrbType};

pub struct LinkTrb {
    addr: u64,
    is_last: bool
}

impl LinkTrb {
    pub fn new(addr: u64, is_last: bool) -> Self {
        Self {
            addr,
            is_last,
        }
    }
}

impl From<LinkTrb> for Trb {
    fn from(trb: LinkTrb) -> Self {
        let control = TrbType::Link.to_control();
        let control = if trb.is_last {control | 0x2} else {control};
        Self {
            parameter: trb.addr,
            status: 0,
            control,
        }
    }
}