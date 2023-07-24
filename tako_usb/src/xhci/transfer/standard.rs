pub struct StandardRequest;
impl StandardRequest {
    pub const GET_STATUS: u8 = 0;
    pub const CLEAR_FEATURE: u8 = 1;
    pub const SET_FEATURE: u8 = 3;
    pub const SET_ADDRESS: u8 = 5;
    pub const GET_DESCRIPTOR: u8 = 6;
    pub const SET_DESCRIPTOR: u8 = 7;
    pub const GET_CONFIGURATION: u8 = 8;
    pub const SET_CONFIGURATION: u8 = 9;
    pub const GET_INTERFACE: u8 = 10;
    pub const SET_INTERFACE: u8 = 11;
    pub const SYNC_FRAME: u8 = 12;
}

pub struct DescriptorType;
impl DescriptorType {
    pub const DEVICE: u8 = 1;
    pub const CONFIGURATION: u8 = 2;
    pub const STRING: u8 = 3;
    pub const INTERFACE: u8 = 4;
    pub const ENDPOINT: u8 = 5;
    pub const DEVICE_QUALIFIER: u8 = 6;
    pub const OTHER_SPEED_CONFIGURATION: u8 = 7;
    pub const INTERFACE_POWER: u8 = 8;
}

pub fn get_descriptor_value(descriptor_type: u8, index: u8) -> u16 {
    (descriptor_type as u16) << 8 | index as u16
}

#[derive(Debug, Copy, Clone)]
pub struct StringIndex(pub u8);

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub bcd_usb: u16,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub max_packet_size: u8,
    pub vendor: u16,
    pub product: u16,
    pub bcd_device: u16,
    pub manufacturer_index: StringIndex,
    pub product_index: StringIndex,
    pub serial_number: StringIndex,
    pub num_configurations: u8,
}