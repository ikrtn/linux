// SPDX-License-Identifier: GPL-2.0

//! I2C subsystem

// I2C Adapter abstractions.
use crate::{
    bindings,
    device,
    error::*,
    prelude::*,
    sync::aref::ARef,
    types::Opaque, //
};

use core::{
    marker::PhantomData,
    ptr::NonNull, //
};

/// The i2c adapter representation.
///
/// This structure represents the Rust abstraction for a C `struct i2c_adapter`. The
/// implementation abstracts the usage of an existing C `struct i2c_adapter` that
/// gets passed from the C side
///
/// # Invariants
///
/// A [`I2cAdapter`] instance represents a valid `struct i2c_adapter` created by the C portion of
/// the kernel.
#[repr(transparent)]
pub struct I2cAdapter<Ctx: device::DeviceContext = device::Normal>(
    Opaque<bindings::i2c_adapter>,
    PhantomData<Ctx>,
);

impl<Ctx: device::DeviceContext> I2cAdapter<Ctx> {
    pub(super) fn as_raw(&self) -> *mut bindings::i2c_adapter {
        self.0.get()
    }
}

impl I2cAdapter {
    /// Returns the I2C Adapter index.
    #[inline]
    pub fn index(&self) -> i32 {
        // SAFETY: `self.as_raw` is a valid pointer to a `struct i2c_adapter`.
        unsafe { (*self.as_raw()).nr }
    }

    /// Gets pointer to an `i2c_adapter` by index.
    pub fn get(index: i32) -> Result<ARef<Self>> {
        // SAFETY: `index` must refer to a valid I2C adapter; the kernel
        // guarantees that `i2c_get_adapter(index)` returns either a valid
        // pointer or NULL. `NonNull::new` guarantees the correct check.
        let adapter = NonNull::new(unsafe { bindings::i2c_get_adapter(index) }).ok_or(ENODEV)?;

        // SAFETY: `adapter` is non-null and points to a live `i2c_adapter`.
        // `I2cAdapter` is #[repr(transparent)], so this cast is valid.
        Ok(unsafe { (&*adapter.as_ptr().cast::<I2cAdapter<device::Normal>>()).into() })
    }
}

// SAFETY: `I2cAdapter` is a transparent wrapper of a type that doesn't depend on
// `I2cAdapter`'s generic argument.
kernel::impl_device_context_deref!(unsafe { I2cAdapter });
kernel::impl_device_context_into_aref!(I2cAdapter);

// SAFETY: Instances of `I2cAdapter` are always reference-counted.
unsafe impl crate::sync::aref::AlwaysRefCounted for I2cAdapter {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        unsafe { bindings::i2c_get_adapter(self.index()) };
    }

    unsafe fn dec_ref(obj: NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::i2c_put_adapter(obj.as_ref().as_raw()) }
    }
}
