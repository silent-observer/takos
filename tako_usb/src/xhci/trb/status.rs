use super::{Trb, TrbType, DataTransferDirection};

pub struct StatusTrb(pub DataTransferDirection);

impl From<StatusTrb> for Trb {
    fn from(trb: StatusTrb) -> Self {
        let control = TrbType::Status.to_control() | 0x20;
        let control = match trb.0 {
            DataTransferDirection::HostToDevice => control,
            DataTransferDirection::DeviceToHost => control | 0x10000,
        };
        Self {
            parameter: 0,
            status: 0,
            control,
        }
    }
}