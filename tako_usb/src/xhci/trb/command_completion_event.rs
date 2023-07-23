use core::mem::transmute;

use super::{Trb, TrbType};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandCompletionCode {
    Invalid = 0,
    Success,
    DataBufferError,
    BabbleDetectedError,
    UsbTransactionError,
    TrbError,
    StallError,
    ResourceError,
    BandwidthError,
    NoSlotsAvailableError,
    InvalidStreamTypeError,
    SlotNotEnabledError,
    EndpointNotEnabledError,
    ShortPacket,
    RingUnderrun,
    RingOverrun,
    VfEventRingFullError,
    ParameterError,
    BandwidthOverrunError,
    ContextStateError,
    NoPingResponseError,
    EventRingFullError,
    IncompatibleDeviceError,
    MissedServiceError,
    CommandRingStopped,
    CommandAborted,
    Stopped,
    StoppedLengthInvalid,
    StoppedShortPacket,
    MaxExitLatencyTooLargeError,
    IsochBufferOverrun = 31,
    EventLostError,
    UndefinedError,
    InvalidStreamIdError,
    SecondaryBandwidthError,
    SplitTransactionError,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandCompletionEventTrb {
    pub addr: u64,
    pub code: CommandCompletionCode,
    pub parameter: u32,
    pub slot_id: u8,
    pub vf_id: u8,
}

impl TryFrom<Trb> for CommandCompletionEventTrb {
    type Error = ();

    fn try_from(trb: Trb) -> Result<Self, Self::Error> {
        if trb.trb_type() != TrbType::CommandCompletionEvent {
            return Err(());
        }

        let code_u8 = (trb.status >> 24) as u8;
        let code = match code_u8 {
            0..=29 | 31..=36 => unsafe {transmute(code_u8)},
            _ => return Err(())
        };

        Ok(Self {
            addr: trb.parameter,
            code,
            parameter: trb.status & 0xFFFFFF,
            slot_id: (trb.control >> 24) as u8,
            vf_id: (trb.control >> 16) as u8,
        })
    }

}