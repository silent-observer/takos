use core::{fmt, mem::transmute};

use alloc::string::{ToString, String};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StandardRequest {
    GetStatus = 0,
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SyncFrame = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower = 8,
}

pub fn get_descriptor_value(descriptor_type: DescriptorType, index: u8) -> u16 {
    (descriptor_type as u16) << 8 | index as u16
}

#[derive(Debug, Copy, Clone, Default)]
pub struct StringIndex(pub u8);

#[derive(Copy, Clone, Default)]
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

impl fmt::Debug for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DeviceDescriptor")?;
        writeln!(f, "  length: {}", self.length)?;
        writeln!(f, "  descriptor_type: 0x{:02X}", self.descriptor_type)?;
        writeln!(f, "  bcd_usb: 0x{:04X}", self.bcd_usb)?;
        writeln!(f, "  class: 0x{:02X}", self.class)?;
        writeln!(f, "  subclass: 0x{:02X}", self.subclass)?;
        writeln!(f, "  protocol: 0x{:02X}", self.protocol)?;
        writeln!(f, "  max_packet_size: {}", self.max_packet_size)?;
        writeln!(f, "  vendor: 0x{:04X}", self.vendor)?;
        writeln!(f, "  product: 0x{:04X}", self.product)?;
        writeln!(f, "  bcd_device: 0x{:04X}", self.bcd_device)?;
        writeln!(f, "  manufacturer_index: {:?}", self.manufacturer_index)?;
        writeln!(f, "  product_index: {:?}", self.product_index)?;
        writeln!(f, "  serial_number: {:?}", self.serial_number)?;
        writeln!(f, "  num_configurations: {}", self.num_configurations)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_index: StringIndex,
    pub attributes: u8,
    pub max_power: u8,
}

impl fmt::Debug for ConfigurationDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ConfigurationDescriptor")?;
        writeln!(f, "  length: {}", self.length)?;
        writeln!(f, "  descriptor_type: 0x{:02X}", self.descriptor_type)?;
        writeln!(f, "  total_length: 0x{:04X}", self.total_length)?;
        writeln!(f, "  num_interfaces: {}", self.num_interfaces)?;
        writeln!(f, "  configuration_value: 0x{:02X}", self.configuration_value)?;
        writeln!(f, "  configuration_index: {:?}", self.configuration_index)?;
        writeln!(f, "  attributes: 0x{:02X}", self.attributes)?;
        writeln!(f, "  max_power: 0x{:02X}", self.max_power)?;
        Ok(())
    }
}

impl ConfigurationDescriptor {
    pub unsafe fn from_slice(data: &[u8]) -> Self {
        unsafe {
            let addr: *const Self = transmute(data.as_ptr());
            assert_eq!((*addr).descriptor_type, DescriptorType::Configuration as u8);
            *addr
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub interface_index: StringIndex,
}

impl fmt::Debug for InterfaceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "InterfaceDescriptor")?;
        writeln!(f, "  length: {}", self.length)?;
        writeln!(f, "  descriptor_type: 0x{:02X}", self.descriptor_type)?;
        writeln!(f, "  interface_number: {}", self.interface_number)?;
        writeln!(f, "  alternate_setting: {}", self.alternate_setting)?;
        writeln!(f, "  num_endpoints: {}", self.num_endpoints)?;
        writeln!(f, "  class: 0x{:02X}", self.class)?;
        writeln!(f, "  subclass: 0x{:02X}", self.subclass)?;
        writeln!(f, "  protocol: 0x{:02X}", self.protocol)?;
        writeln!(f, "  interface_index: {:?}", self.interface_index)?;
        Ok(())
    }
}

impl InterfaceDescriptor {
    pub unsafe fn from_slice(data: &[u8]) -> Self {
        unsafe {
            let addr: *const Self = transmute(data.as_ptr());
            assert_eq!((*addr).descriptor_type, DescriptorType::Interface as u8);
            *addr
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8
}

impl EndpointDescriptor {
    pub unsafe fn from_slice(data: &[u8]) -> Self {
        unsafe {
            let addr: *const Self = transmute(data.as_ptr());
            assert_eq!((*addr).descriptor_type, DescriptorType::Endpoint as u8);
            *addr
        }
    }
}

impl fmt::Debug for EndpointDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "EndpointDescriptor")?;
        writeln!(f, "  length: {}", self.length)?;
        writeln!(f, "  descriptor_type: 0x{:02X}", self.descriptor_type)?;
        writeln!(f, "  endpoint_address: 0x{:02X}", self.endpoint_address)?;
        writeln!(f, "  attributes: 0x{:02X}", self.attributes)?;
        writeln!(f, "  max_packet_size: {}", self.max_packet_size)?;
        writeln!(f, "  interval: {}", self.interval)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct StringDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub data: [u16; 127],
}

impl Default for StringDescriptor {
    fn default() -> Self {
        Self { length: 0, descriptor_type: 0, data: [0; 127] }
    }
}

impl ToString for StringDescriptor {
    fn to_string(&self) -> String {
        let data = &self.data[0..((self.length-2)/2) as usize];
        String::from_utf16(data).unwrap()
    }
}

pub trait Descriptor: Default + Unpin + Clone {}
impl Descriptor for DeviceDescriptor {}
impl Descriptor for ConfigurationDescriptor {}
impl Descriptor for InterfaceDescriptor {}
impl Descriptor for EndpointDescriptor {}
impl Descriptor for StringDescriptor {}