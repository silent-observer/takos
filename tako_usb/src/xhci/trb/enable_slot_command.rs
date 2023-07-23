use super::{Trb, TrbType};

pub struct EnableSlotCommandTrb;

impl From<EnableSlotCommandTrb> for Trb {
    fn from(_: EnableSlotCommandTrb) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: TrbType::EnableSlotCommand.to_control(),
        }
    }
}