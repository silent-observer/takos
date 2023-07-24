use alloc::fmt;
use alloc::fmt::Write;
use alloc::vec::Vec;
use alloc::string::{ToString, String};
use alloc::boxed::Box;
use indenter::{Indented, indented};
use log::info;

use crate::{controller::MemoryInterface, xhci::{transfer::{ControlTransferBuilder, self, standard::StringDescriptor}, trb::{DataTransferDirection, TypeOfRequest, Recipient, TransferEventTrb, CompletionCode}}};

use super::{transfer::standard::{DeviceDescriptor, Descriptor, DescriptorType, StringIndex, ConfigurationDescriptor}, Xhci, PortData};

#[derive(Clone)]
pub struct DeviceInfo {
    pub descriptor: DeviceDescriptor,
    pub manufacturer: String,
    pub product: String,
    pub serial_number: String,
    pub configurations: Vec<ConfigurationInfo>,
}

#[derive(Clone)]
pub struct ConfigurationInfo {
    pub descriptor: ConfigurationDescriptor,
    pub name: String,
}

impl fmt::Debug for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DeviceInfo")?;
        write!(indented(f).with_str("| "), "{:?}", self.descriptor)?;
        writeln!(f, "| manufacturer: {}", self.manufacturer)?;
        writeln!(f, "| product: {}", self.product)?;
        writeln!(f, "| serial_number: {}", self.serial_number)?;
        writeln!(f, "| configurations:")?;
        for config in self.configurations.iter() {
            write!(indented(f).with_str("|   "), "{:?}", config)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ConfigurationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ConfigurationInfo")?;
        write!(indented(f).with_str("| "), "{:?}", self.descriptor)?;
        writeln!(f, "| name: {}", self.name)?;
        Ok(())
    }
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
    async fn get_descriptor<D>(
        &self,
        port_data: &mut PortData,
        descriptor_type: DescriptorType,
        descriptor_index: u8,
        lang_id: Option<u16>
    ) -> Option<D>
    where 
        D: Descriptor
    {
        let mut data = Box::pin(D::default());
        let trbs = ControlTransferBuilder
            ::new(self.mem, port_data.max_packet_size as usize)
            .direction(DataTransferDirection::DeviceToHost)
            .type_of_request(TypeOfRequest::Standard)
            .recipient(Recipient::Device)
            .request(transfer::standard::StandardRequest::GetDescriptor as u8)
            .value(transfer::standard::get_descriptor_value(
                descriptor_type,
                descriptor_index))
            .index(lang_id.unwrap_or(0))
            .with_data(data.as_mut().get_mut())
            .build();
        let response: TransferEventTrb = self.send_transfer(
            port_data.slot_id,
            &mut port_data.transfer_ring,
            &trbs
        ).await.try_into().ok()?;
        if response.code != CompletionCode::Success {
            return None;
        }
        Some(data.as_ref().get_ref().clone())
    }

    pub async fn get_device_descriptor(&self, port_data: &mut PortData) -> Option<DeviceDescriptor> {
        info!("Getting device descriptor at port {}...", port_data.port);
        let d: DeviceDescriptor =
            self.get_descriptor(port_data, DescriptorType::Device, 0, None).await?;
        info!("Got device descriptor for port {}: {:X?}", port_data.port, d);
        Some(d)
    }

    pub async fn get_configuration_descriptor(&self, port_data: &mut PortData, index: u8) -> Option<ConfigurationDescriptor> {
        info!("Getting configuration descriptor at port {}...", port_data.port);
        let d: ConfigurationDescriptor =
            self.get_descriptor(port_data, DescriptorType::Configuration, index, None).await?;
        info!("Got configuration descriptor for port {}: {:X?}", port_data.port, d);
        Some(d)
    }

    pub async fn get_descriptor_string(&self, port_data: &mut PortData, index: StringIndex) -> Option<String> {
        const ENGLISH_LANGID: u16 = 0x0409;
        info!("Getting device descriptor at port {}...", port_data.port);
        let d: StringDescriptor =
            self.get_descriptor(
                port_data,
                DescriptorType::String,
                index.0,
                Some(ENGLISH_LANGID)
            ).await?;
        let s = d.to_string();
        info!("Got string descriptor for port {}: {:X?}", port_data.port, s);
        Some(s)
    }

    async fn examine_configuration(&self, port_data: &mut PortData, index: u8) -> Option<ConfigurationInfo> {
        let descriptor = self.get_configuration_descriptor(port_data, index).await?;
        let name = self.get_descriptor_string(port_data, descriptor.configuration_index).await?;
        let result = ConfigurationInfo {
            descriptor,
            name,
        };
        info!("Got configuration info for port {}:\n{:X?}", port_data.port, result);
        Some(result)
    }

    pub async fn examine_device(&self, port_data: &mut PortData) -> Option<DeviceInfo> {
        let descriptor = self.get_device_descriptor(port_data).await?;
        let manufacturer = self.get_descriptor_string(port_data, descriptor.manufacturer_index).await?;
        let product = self.get_descriptor_string(port_data, descriptor.product_index).await?;
        let serial_number = self.get_descriptor_string(port_data, descriptor.serial_number).await?;
        
        let mut result = DeviceInfo {
            descriptor,
            manufacturer,
            product,
            serial_number,
            configurations: Vec::new(),
        };

        for i in 0..descriptor.num_configurations {
            if let Some(info) = self.examine_configuration(port_data, i).await {
                result.configurations.push(info);
            }
        }
        

        info!("Got device info for port {}:\n{:X?}", port_data.port, result);
        info!("!!!");
        Some(result)
    }
}