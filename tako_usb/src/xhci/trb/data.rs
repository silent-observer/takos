use super::{Trb, TrbType, DataTransferDirection};

pub struct DataTrb {
    addr: u64,
    length: u32,
    td_size: u8,
    dir: DataTransferDirection,
}

impl DataTrb {
    pub fn new(addr: u64, length: u32, td_size: u8, dir: DataTransferDirection) -> Self {
        Self {
            addr,
            length,
            td_size,
            dir
        }
    }
}

impl From<DataTrb> for Trb {
    fn from(trb: DataTrb) -> Self {
        let status = trb.length & 0x1FFFF | (trb.td_size as u32) << 17;
        let control = TrbType::Normal.to_control();
        let control = match trb.dir {
            DataTransferDirection::HostToDevice => control,
            DataTransferDirection::DeviceToHost => control | 0x10000,
        };
        Self {
            parameter: trb.addr,
            status,
            control,
        }
    }
}