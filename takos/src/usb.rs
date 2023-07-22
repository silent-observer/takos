use core::future::pending;

use alloc::boxed::Box;
use tako_usb::{xhci::Xhci, controller::UsbController};
use takobl_api::PHYSICAL_MEMORY_OFFSET;

use crate::paging::PAGE_TABLE;
use crate::println;
use crate::pci::{PciDevice, Header0, BaseAddressRegister, PCI_DEVICES};

pub fn find_usb_host() -> Option<PciDevice> {
    for device in PCI_DEVICES.lock().iter() {
        if device.data.class_code == 0xC && device.data.subclass_code == 0x3 {
            return Some(device.clone());
        }
    }
    None
}

pub async fn usb_driver(usb_host: PciDevice) {
    assert_eq!(usb_host.data.class_code, 0xC);
    assert_eq!(usb_host.data.subclass_code, 0x3);
    let pci_base = match usb_host.header {
        crate::pci::Header::Header0(Header0{
            bars: [
                BaseAddressRegister::Memory(addr),
                ..
            ]
        }) => addr,
        _ => panic!("Cannot find BAR!"),
    };
    let pci_base = pci_base + PHYSICAL_MEMORY_OFFSET;

    let mut usb: Box<dyn UsbController> = match usb_host.data.prog_if {
        0x30 => {
            let xhci = Xhci::new(pci_base as *mut u8, &PAGE_TABLE);
            println!("caplength={}", xhci.registers.capabilities.cap_length().read());
            println!("hciversion={:04X}", xhci.registers.capabilities.hci_version().read());
            println!("hcsparams1={:08X}", xhci.registers.capabilities.hcs_params_1().read());
            println!("hcsparams2={:08X}", xhci.registers.capabilities.hcs_params_2().read());
            println!("hcsparams3={:08X}", xhci.registers.capabilities.hcs_params_3().read());
            println!("hccparams1={:08X}", xhci.registers.capabilities.hcc_params_1().read());
            println!("usbcmd={:08X}", xhci.registers.operational.usbcmd().read());
            println!("usbsts={:08X}", xhci.registers.operational.usbsts().read());
            Box::new(xhci)
        }
        _ => panic!("We don't support USB controller of type {:02X} yet!", usb_host.data.prog_if)
    };

    usb.initialize();
    let event = usb.poll_event();
    println!("Event: {:?}", event);

    let event = usb.poll_event();
    println!("Event: {:?}", event);

    pending::<()>().await;
}