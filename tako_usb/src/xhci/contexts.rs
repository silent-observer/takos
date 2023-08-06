use bitfield::bitfield;

bitfield! {
    #[derive(Debug, Clone, Copy)]
    #[repr(C, align(32))]
    pub struct RawSlotContext([u32]);
    u32;
    pub route_string, set_route_string: 19, 0;
    pub u8, speed, set_speed: 23, 20;
    pub mtt, set_mtt: 25;
    pub hub, set_hub: 26;
    pub u8, context_entries, set_context_entries: 31, 27;

    pub u16, max_exit_latency, set_max_exit_latency: 0x20 + 15, 0x20 + 0;
    pub u8, root_hub_port_number, set_root_hub_port_number: 0x20 + 23, 0x20 + 16;
    pub u8, number_of_ports, set_number_of_ports: 0x20 + 31, 0x20 + 24;

    pub u8, parent_hub_slot_id, set_parent_hub_slot_id: 0x40 + 7, 0x40 + 0;
    pub u8, parent_port_number, set_parent_port_number: 0x40 + 15, 0x40 + 8;
    pub u8, ttt, set_ttt: 0x40 + 17, 0x40 + 16;
    pub u16, interruptor_target, set_interruptor_target: 0x40 + 31, 0x40 + 22;

    pub u8, usb_device_address, set_usb_device_address: 0x60 + 7, 0x60 + 0;
    pub u8, slot_state, set_slot_state: 0x60 + 31, 0x60 + 27;
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
    pub u8, ep_state, set_ep_state: 2, 0;
    pub u8, mult, set_mult: 9, 8;
    pub u8, max_pstreams, set_max_pstreams: 14, 10;
    pub lsa, set_lsa: 15;
    pub u8, interval, set_interval: 23, 16;
    pub u8, max_esit_payload_hi, set_max_esit_payload_hi: 31, 24;

    pub u8, cerr, set_cerr: 0x20 + 2, 0x20 + 1;
    pub u8, ep_type, set_ep_type: 0x20 + 5, 0x20 + 3;
    pub hid, set_hid: 0x20 + 7;
    pub u8, max_burst_size, set_max_burst_size: 0x20 + 15, 0x20 + 8;
    pub u16, max_packet_size, set_max_packet_size: 0x20 + 31, 0x20 + 16;

    pub dcs, set_dcs: 0x40;
    pub u64, _, set_tr_dequeue_ptr: 0x40 + 63, 0x40;

    pub u16, average_trb_length, set_average_trb_length: 0x80 + 15, 0x80 + 0;
    pub u16, max_esit_payload_lo, set_max_esit_payload_lo: 0x80 + 31, 0x80 + 16;
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
pub struct DeviceContextBaseAddressArray(pub [u64; 256]);

impl DeviceContextBaseAddressArray {
    pub fn new() -> Self {
        Self([0; 256])
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InputControlContext {
    pub drop_context_flags: u32,
    pub add_context_flags: u32,
    _reserved1: [u32; 5],
    pub configuration_value: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    _reserved2: u8,
}

impl InputControlContext {
    pub fn new() -> Self {
        Self {
            drop_context_flags: 0,
            add_context_flags: 0,
            _reserved1: [0; 5],
            configuration_value: 0,
            interface_number: 0,
            alternate_setting: 0,
            _reserved2: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(64))]
pub struct InputContext {
    pub control_context: InputControlContext,
    pub slot_context: SlotContext,
    pub endpoint_contexts: [EndpointContext; 31],
}

impl InputContext {
    pub fn new() -> Self {
        Self {
            control_context: InputControlContext::new(),
            slot_context: SlotContext::new(),
            endpoint_contexts: [EndpointContext::new(); 31],
        }
    }
}