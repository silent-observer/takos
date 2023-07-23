pub trait RegisterType {}
pub trait BasicRegisterType {}
impl BasicRegisterType for u8 {}
impl BasicRegisterType for u16 {}
impl BasicRegisterType for u32 {}
impl BasicRegisterType for u64 {}

impl<T:BasicRegisterType> RegisterType for T {}

pub trait RegisterPermissions {}
pub struct RegisterPermissionsReadOnly;
pub struct RegisterPermissionsReadWrite;

impl RegisterPermissions for RegisterPermissionsReadOnly {}
impl RegisterPermissions for RegisterPermissionsReadWrite {}

pub struct RawRegister<T: RegisterType, Permission: RegisterPermissions, const OFFSET: usize> {
    base: *const u8,
    _type: core::marker::PhantomData<T>,
    _permission: core::marker::PhantomData<Permission>,
}

impl<T, Permission, const OFFSET: usize> RawRegister<T, Permission, OFFSET>
where 
    T: RegisterType,
    Permission: RegisterPermissions
{
    pub unsafe fn new(base: *const u8) -> Self {
        Self {
            base,
            _type: core::marker::PhantomData,
            _permission: core::marker::PhantomData,
        }
    }
    pub fn read(&self) -> T {
        unsafe {
            core::ptr::read_volatile(self.base.add(OFFSET) as *const T)
        }
    }
}

impl<T, const OFFSET: usize> RawRegister<T, RegisterPermissionsReadWrite, OFFSET>
where T: RegisterType
{
    pub fn write(&self, val: T) {
        unsafe {
            core::ptr::write_volatile(self.base.add(OFFSET) as *mut T, val)
        }
    }
}

pub type ReadOnlyRegister<T, const OFFSET: usize> = RawRegister<T, RegisterPermissionsReadOnly, OFFSET>;
pub type Register<T, const OFFSET: usize> = RawRegister<T, RegisterPermissionsReadWrite, OFFSET>;

#[macro_export]
macro_rules! register {
    ($name:ident: $t:ty [$base:ident + $offset:literal]) => {
        pub fn $name(&self) -> $crate::register::ReadOnlyRegister<$t, $offset> {
            unsafe{$crate::register::ReadOnlyRegister::new(self.$base)}
        }
    };
    ($name:ident: mut $t:ty [$base:ident + $offset:literal]) => {
        pub fn $name(&self) -> $crate::register::Register<$t, $offset> {
            unsafe{$crate::register::Register::new(self.$base)}
        }
    };
}

pub struct RawRuntimeRegister<T: RegisterType, Permission: RegisterPermissions> {
    addr: *const u8,
    _type: core::marker::PhantomData<T>,
    _permission: core::marker::PhantomData<Permission>,
}

impl<T, Permission> RawRuntimeRegister<T, Permission>
where 
    T: RegisterType,
    Permission: RegisterPermissions
{
    pub unsafe fn new(base: *const u8, offset: usize) -> Self {
        Self {
            addr: base.add(offset),
            _type: core::marker::PhantomData,
            _permission: core::marker::PhantomData,
        }
    }
    pub fn read(&self) -> T {
        unsafe {
            core::ptr::read_volatile(self.addr as *const T)
        }
    }
}

impl<T> RawRuntimeRegister<T, RegisterPermissionsReadWrite>
where T: RegisterType
{
    pub fn write(&self, val: T) {
        unsafe {
            core::ptr::write_volatile(self.addr as *mut T, val)
        }
    }
}

pub type ReadOnlyRuntimeRegister<T> = RawRuntimeRegister<T, RegisterPermissionsReadOnly>;
pub type RuntimeRegister<T> = RawRuntimeRegister<T, RegisterPermissionsReadWrite>;

#[macro_export]
macro_rules! register_array {
    ($name:ident: $t:ty [$base:ident + $size:literal * i + $offset:literal]) => {
        pub fn $name(&self, i: usize) -> $crate::register::ReadOnlyRuntimeRegister<$t> {
            unsafe{$crate::register::ReadOnlyRuntimeRegister::new(self.$base, i * $size + $offset)}
        }
    };
    ($name:ident: mut $t:ty [$base:ident + $size:literal * i + $offset:literal]) => {
        pub fn $name(&self, i: usize) -> $crate::register::RuntimeRegister<$t> {
            unsafe{$crate::register::RuntimeRegister::new(self.$base, i * $size + $offset)}
        }
    };
}