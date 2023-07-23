use super::{Trb, TrbType};

pub struct DisableSlotCommandTrb(pub u8);

impl From<DisableSlotCommandTrb> for Trb {
    fn from(trb: DisableSlotCommandTrb) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: TrbType::DisableSlotCommand.to_control() | (trb.0 as u32) << 24,
        }
    }
}