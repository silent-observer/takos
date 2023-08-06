use super::{Trb, TrbType};

#[derive(Debug, Clone, Copy)]
pub struct AddressDeviceCommandTrb {
    slot_id: u8,
    input_context: u64,
    bsr: bool,
}

impl AddressDeviceCommandTrb {
    pub fn new(slot_id: u8, input_context: u64, bsr: bool) -> Self {
        Self {
            slot_id,
            input_context,
            bsr,
        }
    }
}

impl From<AddressDeviceCommandTrb> for Trb {
    fn from(trb: AddressDeviceCommandTrb) -> Self {
        let control = TrbType::AddressDeviceCommand.to_control();
        let control = control | (trb.slot_id as u32) << 24;
        let control = control | (trb.bsr as u32) << 9;
        Self {
            parameter: trb.input_context,
            status: 0,
            control,
        }
    }
}