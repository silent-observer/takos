use alloc::{fmt, vec};
use alloc::fmt::Write;
use alloc::vec::Vec;
use alloc::string::{ToString, String};
use alloc::boxed::Box;
use indenter::indented;
use log::info;

use crate::{controller::MemoryInterface, xhci::{transfer::{ControlTransferBuilder, self, standard::StringDescriptor}, trb::{DataTransferDirection, TypeOfRequest, Recipient, TransferEventTrb, CompletionCode}}};

use super::transfer::standard::{InterfaceDescriptor, EndpointDescriptor};
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
    pub interfaces: Vec<InterfaceInfo>
}

#[derive(Clone)]
pub struct InterfaceInfo {
    pub descriptor: InterfaceDescriptor,
    pub name: String,
    pub endpoints: Vec<EndpointDescriptor>
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
        writeln!(f, "| interfaces:")?;
        for config in self.interfaces.iter() {
            write!(indented(f).with_str("|   "), "{:?}", config)?;
        }
        Ok(())
    }
}

impl fmt::Debug for InterfaceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "InterfaceInfo")?;
        write!(indented(f).with_str("| "), "{:?}", self.descriptor)?;
        writeln!(f, "| name: {}", self.name)?;
        writeln!(f, "| endpoints:")?;
        for config in self.endpoints.iter() {
            write!(indented(f).with_str("|   "), "{:?}", config)?;
        }
        Ok(())
    }
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
    async fn get_descriptor_helper<T: ?Sized>(
        &self,
        port_data: &mut PortData,
        descriptor_type: DescriptorType,
        descriptor_index: u8,
        lang_id: Option<u16>,
        data: &mut T
    ) -> Option<TransferEventTrb> {
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
            .with_data(data)
            .build();
        info!("TRBS: {:X?}", trbs);
        info!("Slot context: {:X?}", port_data.device_context.slot_context);
        info!("Endpoint context: {:X?}", port_data.device_context.endpoint_contexts[0]);
        let response = self.send_transfer(
            port_data.slot_id,
            &mut port_data.transfer_ring,
            &trbs
        ).await.try_into().ok();
        info!("Port {} got response (descriptor): {:X?}", port_data.port, response);
        let portsc = self.registers.operational.portsc(port_data.port as usize).read();
        info!("PortSC: {:X?}", portsc);
        response
    }
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
        let response = 
            self.get_descriptor_helper(
                port_data,
                descriptor_type,
                descriptor_index,
                lang_id,
                data.as_mut().get_mut()
            ).await?;
        if response.code != CompletionCode::Success {
            return None;
        }
        Some(data.as_ref().get_ref().clone())
    }

    async fn get_descriptor_raw(
        &self,
        length: usize,
        port_data: &mut PortData,
        descriptor_type: DescriptorType,
        descriptor_index: u8,
        lang_id: Option<u16>,
    ) -> Option<Vec<u8>>
    {
        let mut data = vec![0u8; length];
        let response = 
            self.get_descriptor_helper(
                port_data,
                descriptor_type,
                descriptor_index,
                lang_id,
                data.as_mut_slice()
            ).await?;
        if response.code != CompletionCode::Success {
            return None;
        }
        Some(data)
    }

    pub async fn get_short_descriptor(
        &self,
        port_data: &mut PortData,
    ) -> Option<[u8; 8]>
    {
        let mut data = [0u8; 8];
        let response = 
            self.get_descriptor_helper(
                port_data,
                DescriptorType::Device,
                0,
                None,
                &mut data
            ).await?;
        if response.code != CompletionCode::Success {
            return None;
        }
        Some(data)
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
        if index.0 == 0 {
            return Some("<none>".to_string());
        }

        const ENGLISH_LANGID: u16 = 0x0409;
        info!("Getting string descriptor at port {}...", port_data.port);
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
        let short_descriptor = self.get_configuration_descriptor(port_data, index).await?;
        assert_eq!(short_descriptor.descriptor_type, DescriptorType::Configuration as u8);
        let total_length = short_descriptor.total_length;
        
        
        let data = 
            self.get_descriptor_raw(
                total_length as usize,
                port_data,
                DescriptorType::Configuration,
                index,
                None
            ).await?;

        let mut offset = 0;
        let mut result: Option<ConfigurationInfo> = None;
        while offset < data.len() {
            let descriptor_type = data[offset+1];
            if descriptor_type == DescriptorType::Configuration as u8 {
                let descriptor = unsafe {
                    ConfigurationDescriptor::from_slice(&data[offset..])
                };
                let name = self.get_descriptor_string(port_data, descriptor.configuration_index).await?;
                result = Some(ConfigurationInfo {
                    descriptor,
                    name,
                    interfaces: Vec::new(),
                })
            } else if descriptor_type == DescriptorType::Interface as u8 {
                let descriptor = unsafe {
                    InterfaceDescriptor::from_slice(&data[offset..])
                };
                let name = self.get_descriptor_string(port_data, descriptor.interface_index).await?;
                let info = InterfaceInfo {
                    descriptor,
                    name,
                    endpoints: Vec::new(),
                };
                result.as_mut()
                    .unwrap()
                    .interfaces
                    .push(info);
            } else if descriptor_type == DescriptorType::Endpoint as u8 {
                let descriptor = unsafe {
                    EndpointDescriptor::from_slice(&data[offset..])
                };
                result.as_mut()
                    .unwrap()
                    .interfaces
                    .last_mut()
                    .unwrap()
                    .endpoints
                    .push(descriptor);
            }
            offset += data[offset] as usize;
        }

        // info!("Got configuration info for port {}:\n{:X?}", port_data.port, result);
        result
    }

    pub async fn examine_device(&self, port_data: &mut PortData) -> Option<DeviceInfo> {
        // for i in 0..5 {
        //     info!("Clearing halt {} on port {}", i, port_data.port);
        //     self.clear_halted(port_data).await;
        // }

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

    async fn clear_halted(&self, port_data: &mut PortData) {
        info!("Clearing halted state for port {}", port_data.port);
        let trbs = ControlTransferBuilder::<_, ()>
            ::new(self.mem, port_data.max_packet_size as usize)
            .direction(DataTransferDirection::HostToDevice)
            .type_of_request(TypeOfRequest::Standard)
            .recipient(Recipient::Endpoint)
            .request(transfer::standard::StandardRequest::ClearFeature as u8)
            .value(0)
            .index(0)
            .build();
        info!("TRBS: {:X?}", trbs);
        info!("Slot context: {:X?}", port_data.device_context.slot_context);
        info!("Endpoint context: {:X?}", port_data.device_context.endpoint_contexts[0]);
        let response: Option<TransferEventTrb> = self.send_transfer(
            port_data.slot_id,
            &mut port_data.transfer_ring,
            &trbs
        ).await.try_into().ok();
        info!("Port {} got response (halt clear): {:X?}", port_data.port, response);
        let portsc = self.registers.operational.portsc(port_data.port as usize).read();
        info!("PortSC: {:X?}", portsc);
    }
}