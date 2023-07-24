use super::{Trb, TrbType, DataTransferDirection};

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeOfRequest {
    Standard = 0,
    Class = 1,
    Vendor = 2,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}

pub struct SetupTrb {
    pub dir: DataTransferDirection,
    pub type_of_request: TypeOfRequest,
    pub recipient: Recipient,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16
}

impl From<SetupTrb> for Trb {
    fn from(trb: SetupTrb) -> Self {
        let parameter = (trb.recipient as u8 as u64)
            | ((trb.type_of_request as u8 as u64) << 5)
            | (trb.dir as u8 as u64) << 7
            | (trb.request as u64) << 8
            | (trb.value as u64) << 16
            | (trb.index as u64) << 32
            | (trb.length as u64) << 48;
        let trt = if trb.length == 0 {0}
            else {
                if trb.dir == DataTransferDirection::HostToDevice {2}
                else {3}
            };
        Self {
            parameter,
            status: 0x8,
            control: 0x40 | TrbType::Setup.to_control() | (trt << 16),
        }
    }
}