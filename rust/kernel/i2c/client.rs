// SPDX-License-Identifier: GPL-2.0

//! I2C subsystem

// I2C Client abstractions.
use crate::{
    container_of,
    device,
    devres::Devres,
    error::*,
    i2c::adapter::I2cAdapter,
    prelude::*,
    sync::aref::AlwaysRefCounted,
    types::Opaque, //
};

use core::{
    marker::PhantomData,
    mem::offset_of,
    ptr::{
        from_ref,
        NonNull, //
    }, //
};

/// The i2c board info representation
///
/// This structure represents the Rust abstraction for a C `struct i2c_board_info` structure,
/// which is used for manual I2C client creation.
#[repr(transparent)]
pub struct I2cBoardInfo(bindings::i2c_board_info);

impl I2cBoardInfo {
    const I2C_TYPE_SIZE: usize = 20;
    /// Create a new [`I2cBoardInfo`] for a kernel driver.
    #[inline(always)]
    pub const fn new(type_: &'static CStr, addr: u16) -> Self {
        let src = type_.to_bytes_with_nul();
        build_assert!(src.len() <= Self::I2C_TYPE_SIZE, "Type exceeds 20 bytes");
        let mut i2c_board_info: bindings::i2c_board_info = pin_init::zeroed();
        let mut i: usize = 0;
        while i < src.len() {
            i2c_board_info.type_[i] = src[i];
            i += 1;
        }

        i2c_board_info.addr = addr;
        Self(i2c_board_info)
    }

    fn as_raw(&self) -> *const bindings::i2c_board_info {
        from_ref(&self.0)
    }
}

/// The i2c client representation.
///
/// This structure represents the Rust abstraction for a C `struct i2c_client`. The
/// implementation abstracts the usage of an existing C `struct i2c_client` that
/// gets passed from the C side
///
/// # Invariants
///
/// A [`I2cClient`] instance represents a valid `struct i2c_client` created by the C portion of
/// the kernel.
#[repr(transparent)]
pub struct I2cClient<Ctx: device::DeviceContext = device::Normal>(
    Opaque<bindings::i2c_client>,
    PhantomData<Ctx>,
);

impl<Ctx: device::DeviceContext> I2cClient<Ctx> {
    pub(super) fn as_raw(&self) -> *mut bindings::i2c_client {
        self.0.get()
    }
}

// SAFETY: `I2cClient` is a transparent wrapper of `struct i2c_client`.
// The offset is guaranteed to point to a valid device field inside `I2cClient`.
unsafe impl<Ctx: device::DeviceContext> device::AsBusDevice<Ctx> for I2cClient<Ctx> {
    const OFFSET: usize = offset_of!(bindings::i2c_client, dev);
}

// SAFETY: `I2cClient` is a transparent wrapper of a type that doesn't depend on
// `I2cClient`'s generic argument.
kernel::impl_device_context_deref!(unsafe { I2cClient });
kernel::impl_device_context_into_aref!(I2cClient);

// SAFETY: Instances of `I2cClient` are always reference-counted.
unsafe impl AlwaysRefCounted for I2cClient {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        unsafe { bindings::get_device(self.as_ref().as_raw()) };
    }

    unsafe fn dec_ref(obj: NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::put_device(&raw mut (*obj.as_ref().as_raw()).dev) }
    }
}

impl<Ctx: device::DeviceContext> AsRef<device::Device<Ctx>> for I2cClient<Ctx> {
    fn as_ref(&self) -> &device::Device<Ctx> {
        let raw = self.as_raw();
        // SAFETY: By the type invariant of `Self`, `self.as_raw()` is a pointer to a valid
        // `struct i2c_client`.
        let dev = unsafe { &raw mut (*raw).dev };

        // SAFETY: `dev` points to a valid `struct device`.
        unsafe { device::Device::from_raw(dev) }
    }
}

impl<Ctx: device::DeviceContext> TryFrom<&device::Device<Ctx>> for &I2cClient<Ctx> {
    type Error = kernel::error::Error;

    fn try_from(dev: &device::Device<Ctx>) -> Result<Self, Self::Error> {
        // SAFETY: By the type invariant of `Device`, `dev.as_raw()` is a valid pointer to a
        // `struct device`.
        if unsafe { bindings::i2c_verify_client(dev.as_raw()).is_null() } {
            return Err(EINVAL);
        }

        // SAFETY: We've just verified that the type of `dev` equals to
        // `bindings::i2c_client_type`, hence `dev` must be embedded in a valid
        // `struct i2c_client` as guaranteed by the corresponding C code.
        let idev = unsafe { container_of!(dev.as_raw(), bindings::i2c_client, dev) };

        // SAFETY: `idev` is a valid pointer to a `struct i2c_client`.
        Ok(unsafe { &*idev.cast() })
    }
}

// SAFETY: A `I2cClient` is always reference-counted and can be released from any thread.
unsafe impl Send for I2cClient {}

// SAFETY: `I2cClient` can be shared among threads because all methods of `I2cClient`
// (i.e. `I2cClient<Normal>) are thread safe.
unsafe impl Sync for I2cClient {}

/// The registration of an i2c client device.
///
/// This type represents the registration of a [`struct i2c_client`]. When an instance of this
/// type is dropped, its respective i2c client device will be unregistered from the system.
///
/// # Invariants
///
/// `self.0` always holds a valid pointer to an initialized and registered
/// [`struct i2c_client`].
#[repr(transparent)]
pub struct Registration(NonNull<bindings::i2c_client>);

impl Registration {
    /// The C `i2c_new_client_device` function wrapper for manual I2C client creation.
    pub fn new<'a>(
        i2c_adapter: &I2cAdapter,
        i2c_board_info: &I2cBoardInfo,
        parent_dev: &'a device::Device<device::Bound>,
    ) -> impl PinInit<Devres<Self>, Error> + 'a {
        Devres::new(parent_dev, Self::try_new(i2c_adapter, i2c_board_info))
    }

    fn try_new(i2c_adapter: &I2cAdapter, i2c_board_info: &I2cBoardInfo) -> Result<Self> {
        // SAFETY: the kernel guarantees that `i2c_new_client_device()` returns either a valid
        // pointer or NULL. `from_err_ptr` separates errors. Following `NonNull::new`
        // checks for NULL.
        let raw_dev = from_err_ptr(unsafe {
            bindings::i2c_new_client_device(i2c_adapter.as_raw(), i2c_board_info.as_raw())
        })?;

        let dev_ptr = NonNull::new(raw_dev).ok_or(ENODEV)?;

        Ok(Self(dev_ptr))
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        // SAFETY: `Drop` is only called for a valid `Registration`, which by invariant
        // always contains a non-null pointer to an `i2c_client`.
        unsafe { bindings::i2c_unregister_device(self.0.as_ptr()) }
    }
}

// SAFETY: A `Registration` of a `struct i2c_client` can be released from any thread.
unsafe impl Send for Registration {}

// SAFETY: `Registration` offers no interior mutability (no mutation through &self
// and no mutable access is exposed)
unsafe impl Sync for Registration {}
