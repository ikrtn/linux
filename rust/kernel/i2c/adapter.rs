// SPDX-License-Identifier: GPL-2.0

//! I2C subsystem

// I2C Adapter abstractions.
use crate::{
    bindings,
    device,
    devres::Devres,
    error::*,
    i2c::algo::*,
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

    /// Convert a raw C `struct i2c_adapter` pointer to a `&'a I2cAdapter`.
    pub(super) fn from_raw<'a>(ptr: *mut bindings::i2c_adapter) -> &'a Self {
        // SAFETY: Callers must ensure that `ptr` is valid, non-null, and has a non-zero reference
        // count, i.e. it must be ensured that the reference count of the C `struct i2c_adapter`
        // `ptr` points to can't drop to zero, for the duration of this function call and the entire
        // duration when the returned reference exists.
        unsafe { &*ptr.cast() }
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

impl<Ctx: device::DeviceContext> AsRef<device::Device<Ctx>> for I2cAdapter<Ctx> {
    fn as_ref(&self) -> &device::Device<Ctx> {
        let raw = self.as_raw();
        // SAFETY: By the type invariant of `Self`, `self.as_raw()` is a pointer to a valid
        // `struct i2c_adapter`.
        let dev = unsafe { &raw mut (*raw).dev };

        // SAFETY: `dev` points to a valid `struct device`.
        unsafe { device::Device::from_raw(dev) }
    }
}

/// Options for creating an I2C adapter device.
pub struct I2cAdapterOptions {
    /// The name of the I2C adapter device.
    pub name: &'static CStr,
}

impl I2cAdapterOptions {
    /// Create a raw `struct i2c_adapter` ready for registration.
    pub const fn as_raw<T: I2cAlgorithm>(self) -> bindings::i2c_adapter {
        let mut adapter: bindings::i2c_adapter = pin_init::zeroed();
        // TODO: make it some other way... this looks like shit
        let src = self.name.to_bytes_with_nul();
        let mut i: usize = 0;
        while i < src.len() {
            adapter.name[i] = src[i];
            i += 1;
        }
        adapter.algo = I2cAlgorithmVTable::<T>::build();

        adapter
    }
}

/// A registration of a I2C Adapter.
///
/// # Invariants
///
/// - `inner` contains a `struct i2c_adapter` that is registered using
///   `i2c_add_adapter()`.
/// - This registration remains valid for the entire lifetime of the
///   [`i2c::adapter::Registration<T>`] instance.
/// - Deregistration occurs exactly once in [`Drop`] via `i2c_del_adapter()`.
/// - `inner` wraps a valid, pinned `i2c_adapter` created using
///   [`I2cAdapterOptions::as_raw`].
#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Registration<T> {
    #[pin]
    inner: Opaque<bindings::i2c_adapter>,
    t_: PhantomData<T>,
}

impl<T: I2cAlgorithm> Registration<T> {
    /// Register an I2C adapter.
    pub fn register<'a>(
        parent_dev: &'a device::Device<device::Bound>,
        opts: I2cAdapterOptions,
    ) -> impl PinInit<Devres<Self>, Error> + 'a
    where
        T: 'a,
    {
        Devres::new(parent_dev, Self::new(parent_dev, opts))
    }

    fn new<'a>(
        parent_dev: &'a device::Device<device::Bound>,
        opts: I2cAdapterOptions,
    ) -> impl PinInit<Self, Error> + use<'a, T>
    where
        T: 'a,
    {
        try_pin_init! { Self {
            inner <- Opaque::try_ffi_init(move |slot: *mut bindings::i2c_adapter| {
                // SAFETY: The initializer can write to the provided `slot`.
                unsafe {slot.write(opts.as_raw::<T>()) };

                // SAFETY: `slot` is valid from the initializer; `parent_dev` outlives the adapter.
                unsafe { (*slot).dev.parent = parent_dev.as_raw() };

                // SAFETY: the `struct i2c_adapter` was just created in slot. The adapter will
                // get unregistered before `slot` is deallocated because the memory is pinned and
                // the destructor of this type deallocates the memory.
                // INVARIANT: If this returns `Ok(())`, then the `slot` will contain a registered
                // i2c adapter.
                to_result(unsafe {bindings::i2c_add_adapter(slot)})
            }),
            t_: PhantomData,
            }
        }
    }
}

#[pinned_drop]
impl<T> PinnedDrop for Registration<T> {
    fn drop(self: Pin<&mut Self>) {
        // SAFETY: We know that the device is registered by the type invariants.
        unsafe { bindings::i2c_del_adapter(self.inner.get()) };
    }
}

// SAFETY: A `Registration` of a `struct i2c_client` can be released from any thread.
unsafe impl<T> Send for Registration<T> {}

// SAFETY: `Registration` offers no interior mutability (no mutation through &self
// and no mutable access is exposed)
unsafe impl<T> Sync for Registration<T> {}
