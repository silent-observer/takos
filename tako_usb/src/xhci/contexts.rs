use alloc::boxed::Box;
use bitfield::bitfield;

bitfield! {
    #[derive(Debug, Clone, Copy)]
    #[repr(C, align(32))]
    pub struct RawSlotContext([u32]);
    u32;
    route_string, _: 19, 0;
    mtt, _: 25;
    hub, _: 26;

    u16, max_exit_latency, _: 0x20 + 15, 0x20 + 0;
    u8, root_hub_port_number, _: 0x20 + 23, 0x20 + 16;
    u8, number_of_ports, _: 0x20 + 31, 0x20 + 24;

    u8, parent_hub_slot_id, _: 0x40 + 7, 0x40 + 0;
    u8, parent_port_number, _: 0x40 + 15, 0x40 + 8;
    u8, ttt, _: 0x40 + 17, 0x40 + 16;
    u16, interruptor_target, _: 0x40 + 31, 0x40 + 22;

    u8, usb_device_address, _: 0x60 + 7, 0x60 + 0;
    u8, slot_state, _: 0x60 + 31, 0x60 + 27;
}

pub type SlotContext = RawSlotContext<[u32; 8]>;
impl SlotContext {
    fn new() -> Self {
        Self([0; 8])
    }
}

bitfield! {
    #[derive(Debug, Clone, Copy)]
    #[repr(C, align(32))]
    pub struct RawEndpointContext([u32]);
    u32;
    u8, ep_state, _: 2, 0;
    u8, mult, _: 9, 8;
    u8, max_pstreams, _: 14, 10;
    lsa, _: 15;
    u8, interval, _: 23, 16;
    u8, max_esit_payload_hi, _: 31, 24;

    u8, cerr, _: 0x20 + 2, 0x20 + 1;
    u8, ep_type, _: 0x20 + 5, 0x20 + 3;
    hid, _: 0x20 + 7;
    u8, max_burst_size, _: 0x20 + 15, 0x20 + 8;
    u16, max_packet_size, _: 0x20 + 31, 0x20 + 16;

    dcs, _: 0x40;
    u64, tr_dequeue_ptr, _: 0x40 + 63, 0x40 + 4;

    u16, average_trb_length, _: 0x80 + 15, 0x80 + 0;
    u16, max_esit_payload_lo, _: 0x80 + 31, 0x80 + 16;
}

pub type EndpointContext = RawEndpointContext<[u32; 8]>;
impl EndpointContext {
    fn new() -> Self {
        Self([0; 8])
    }
}

#[repr(C, align(64))]
pub struct DeviceContext {
    pub slot_context: SlotContext,
    pub endpoint_contexts: [EndpointContext; 31],
}

impl DeviceContext {
    pub fn new() -> Self {
        DeviceContext {
            slot_context: SlotContext::new(),
            endpoint_contexts: [EndpointContext::new(); 31],
        }
    }
}

#[repr(C, align(64))]
pub struct DeviceContextBaseAddressArray([u64; 256]);

impl DeviceContextBaseAddressArray {
    pub fn new() -> Self {
        Self([0; 256])
    }
}