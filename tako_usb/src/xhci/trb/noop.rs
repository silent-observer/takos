use super::{Trb, TrbType};

pub struct NoOpTrb;

impl From<NoOpTrb> for Trb {
    fn from(_: NoOpTrb) -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: TrbType::NoOp.to_control(),
        }
    }
}