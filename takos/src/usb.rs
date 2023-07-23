use core::future::{pending, self};

use alloc::boxed::Box;
use log::info;
use tako_usb::xhci::trb::Trb;
use tako_usb::{xhci::Xhci, controller::UsbController};
use takobl_api::PHYSICAL_MEMORY_OFFSET;
use tako_async::timer::Timer;

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

    let mut usb= match usb_host.data.prog_if {
        0x30 => {
            let xhci = Xhci::new(pci_base as *mut u8, &PAGE_TABLE);
            info!("caplength={}", xhci.registers.capabilities.cap_length().read());
            info!("hciversion={:04X}", xhci.registers.capabilities.hci_version().read());
            info!("hcsparams1={:08X}", xhci.registers.capabilities.hcs_params_1().read());
            info!("hcsparams2={:08X}", xhci.registers.capabilities.hcs_params_2().read());
            info!("hcsparams3={:08X}", xhci.registers.capabilities.hcs_params_3().read());
            info!("hccparams1={:08X}", xhci.registers.capabilities.hcc_params_1().read());
            info!("usbcmd={:08X}", xhci.registers.operational.usbcmd().read());
            info!("usbsts={:08X}", xhci.registers.operational.usbsts().read());
            xhci
        }
        _ => panic!("We don't support USB controller of type {:02X} yet!", usb_host.data.prog_if)
    };

    let pagesize = usb.registers.operational.pagesize().read();
    info!("pagesize={:08X}", pagesize);

    usb.initialize();
    let running = usb.run();

    let other = async {
        let erstba = usb.registers.runtime.erstba(0).read();
        info!("erstba={:08X}", erstba);
        let erdp = usb.registers.runtime.erdp(0).read();
        info!("erdp={:08X}", erdp);
        let erstsz = usb.registers.runtime.erstsz(0).read();
        info!("erstsz={:08X}", erstsz);
        // let erst = unsafe {
        //     ((erstba + PHYSICAL_MEMORY_OFFSET) as *mut Erst).as_ref().unwrap()
        // };
        // println!("erst[0]={:08X}", erst.0[0].base_addr);
        let status = usb.registers.operational.usbsts().read();
        info!("status={:08X}", status);

        usb.initialize_devices().await;

        // usb.command_ring.enqueue_trb(Trb::noop_command());
        // usb.registers.doorbell.ring_host();
        for _ in 0..4 {
            let command = usb.send_command(Trb::noop_command());
            let trb = *usb.command_ring.lock().first_trb();
            info!("Sent command: {:X?}", trb);

            let trb = command.await;
            info!("Response: {:X?}", trb);
            Timer::new(10).await;
        }
    };

    futures::future::join(running, other).await;

    pending::<()>().await;
}