use crate::register_array;
use crate::register::Register;
use crate::register;

pub struct CapabilityRegisters {
    base: *mut u8
}

impl CapabilityRegisters {
    unsafe fn new(base: *mut u8) -> Self {
        Self {
            base
        }
    }

    register!(cap_length: u8 [base+0x00]);
    register!(hci_version: u16 [base+0x02]);
    register!(hcs_params_1: u32 [base+0x04]);
    register!(hcs_params_2: u32 [base+0x08]);
    register!(hcs_params_3: u32 [base+0x0C]);
    register!(hcc_params_1: u32 [base+0x10]);
    register!(db_off: u32 [base+0x14]);
    register!(rts_off: u32 [base+0x18]);
    register!(hcc_params_2: u32 [base+0x1C]);
}

pub struct OperationalRegisters {
    base: *mut u8
}

impl OperationalRegisters {
    unsafe fn new(base: *mut u8) -> Self {
        Self {
            base
        }
    }

    register!(usbcmd: mut u32 [base+0x00]);
    register!(usbsts: mut u32 [base+0x04]);
    register!(pagesize: mut u32 [base+0x08]);
    register!(dnctrl: mut u32 [base+0x14]);
    register!(crcr: mut u64 [base+0x18]);
    register!(dcbaap: mut u64 [base+0x30]);
    register!(config: mut u32 [base+0x38]);

    register_array!(portsc: mut u32 [base+0x10*i+0x3F0]);
    register_array!(portpm: mut u32 [base+0x10*i+0x3F4]);
    register_array!(portli: u32 [base+0x10*i+0x3F8]);
    register_array!(porthlmpc: mut u32 [base+0x10*i+0x3FC]);
}

pub struct RuntimeRegisters {
    base: *mut u8
}

impl RuntimeRegisters {
    unsafe fn new(base: *mut u8) -> Self {
        Self {
            base
        }
    }

    register!(mfindex: u32 [base+0x00]);
    register_array!(iman: mut u32 [base+0x20*i+0x20]);
    register_array!(imod: mut u32 [base+0x20*i+0x24]);
    register_array!(erstsz: mut u32 [base+0x20*i+0x28]);
    register_array!(erstba: mut u64 [base+0x20*i+0x30]);
    register_array!(erdp: mut u64 [base+0x20*i+0x38]);
}

pub struct DoorbellRegisters {
    base: *mut u8
}

impl DoorbellRegisters {
    unsafe fn new(base: *mut u8) -> Self {
        Self {
            base
        }
    }

    pub fn ring_host(&self) {
        unsafe{Register::<u32, 0x00>::new(self.base).write(0)};
    }
}

pub struct Registers {
    pub capabilities: CapabilityRegisters,
    pub operational: OperationalRegisters,
    pub runtime: RuntimeRegisters,
    pub doorbell: DoorbellRegisters,
}

impl Registers {
    pub unsafe fn new(base: *mut u8) -> Self {
        unsafe {
            let capabilities = CapabilityRegisters::new(base);
            let op_base = base.add(capabilities.cap_length().read() as usize);
            let operational = OperationalRegisters::new(op_base);
            let runtime_base = base.add(capabilities.rts_off().read() as usize);
            let runtime = RuntimeRegisters::new(runtime_base);
            let doorbell_base = base.add(capabilities.db_off().read() as usize);
            let doorbell = DoorbellRegisters::new(doorbell_base);
            Self { capabilities, operational, runtime, doorbell }
        }
    }
}