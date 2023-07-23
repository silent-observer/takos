use super::{Trb, TrbType};

pub struct NoOpCommandTrb;

impl From<NoOpCommandTrb> for Trb {
    fn from(_: NoOpCommandTrb) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: TrbType::NoOpCommand.to_control(),
        }
    }
}