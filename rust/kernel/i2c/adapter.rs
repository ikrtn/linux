use crate::{
    bindings,
    device,
    devres::Devres,
    error::*,
    i2c::algo::*,
    prelude::*,
    types::Opaque, //
};

use core::{
    marker::PhantomData,
    ptr::NonNull, //
};

use kernel::types::ARef;

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
    ///
    /// # Safety
    ///
    /// Callers must ensure that `ptr` is valid, non-null, and has a non-zero reference count,
    /// i.e. it must be ensured that the reference count of the C `struct i2c_adapter` `ptr` points to
    /// can't drop to zero, for the duration of this function call and the entire duration when the
    /// returned reference exists.
    pub(super) fn from_raw<'a>(ptr: *mut bindings::i2c_adapter) -> &'a Self {
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
unsafe impl crate::types::AlwaysRefCounted for I2cAdapter {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        unsafe { bindings::i2c_get_adapter(self.index()) };
    }

    unsafe fn dec_ref(obj: NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::i2c_put_adapter(obj.as_ref().as_raw()) }
    }
}

pub struct I2cAdapterOptions {
    /// The name of the miscdevice.
    pub name: &'static CStr,
}

impl I2cAdapterOptions {
    pub const fn as_raw<T: I2cAlgorithm>(self) -> bindings::i2c_adapter {
        let mut adapter: bindings::i2c_adapter = pin_init::zeroed();
        // TODO: make it some other way... this looks like shit
        let src = self.name.as_bytes_with_nul();
        let mut i: usize = 0;
        while i < src.len() {
            adapter.name[i] = src[i];
            i += 1;
        }
        adapter.algo = I2cAlgorithmVTable::<T>::build();

        adapter
    }
}

#[repr(transparent)]
#[pin_data]
pub struct Registration<T> {
    #[pin]
    inner: Opaque<bindings::i2c_adapter>,
    t_: PhantomData<T>,
}

impl<T: I2cAlgorithm> Registration<T> {
    pub fn register<'a>(
        parent_dev: &'a device::Device<device::Bound>,
        opts: I2cAdapterOptions,
    ) -> impl PinInit<Devres<Self>, Error> + 'a
    where
        T: 'a,
    {
        Devres::new(parent_dev, Self::new(opts))
    }

    fn new(opts: I2cAdapterOptions) -> impl PinInit<Self, Error> {
        try_pin_init! { Self {
            inner <- Opaque::try_ffi_init(move |slot: *mut bindings::i2c_adapter| {
                    unsafe {slot.write(opts.as_raw::<T>()) };

                    to_result(unsafe {bindings::i2c_add_adapter(slot)})
                }),
            t_: PhantomData,
            }
        }
    }
}

// SAFETY: A `Registration` of a `struct i2c_client` can be released from any thread.
unsafe impl<T> Send for Registration<T> {}

// SAFETY: `Registration` offers no interior mutability (no mutation through &self
// and no mutable access is exposed)
unsafe impl<T> Sync for Registration<T> {}
