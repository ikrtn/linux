use crate::{
    bindings,
    device,
    devres::Devres,
    error::*,
    i2c::algo::*,
    prelude::*,
    types:: Opaque, //
};

use core::{
    marker::PhantomData, //
};

struct I2cAdapterOptions{
    /// The name of the miscdevice.
    pub name: &'static CStr,
}

impl I2cAdapterOptions {
    pub const fn from_raw<T: I2cAlgorithm>(self) -> bindings::i2c_adapter {
        let mut adapter: bindings::i2c_adapter = pin_init::zeroed();
        adapter.name = crate::str::as_char_ptr_in_const_context(self.name);
        adapter.algo = I2cAlgorithmVTable::<T>::build();
        
        adapter
    }
}

#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Registration<T> {
    #[pin]
    inner: bindings::i2c_adapter,
    t_: PhantomData<T>
}

impl<T: I2cAlgorithm> Registration<T> {
    pub fn register<'a>(
        parent_dev: &'a device::Device<device::Bound>,
        opts: I2cAdapterOptions
    ) -> impl PinInit<Devres<Self>, Error> + 'a where T: 'a{
        Devres::new(parent_dev, Self::new(opts))
    }

    fn new(opts: I2cAdapterOptions) -> impl PinInit<Self, Error> {
        try_pin_init! { Self {
            inner <- Opaque::try_ffi_init(move |slot: *mut bindings::i2c_adapter| {
                    unsafe {slot.write(opts.from_raw::<T>()) };

                    to_result(unsafe {bindings::i2c_add_adapter(slot)})
                }),
            t_: PhantomData,
            }
        }
    }
}

impl<T> Drop for Registration<T> {
    fn drop(&mut self) {
        // SAFETY: `Drop` is only called for a valid `Registration`, which by invariant
        // always contains a non-null pointer to an `i2c_client`.
        unsafe { bindings::i2c_unregister_device(self.0.as_ptr()) }
    }
}

// SAFETY: A `Registration` of a `struct i2c_client` can be released from any thread.
unsafe impl<T> Send for Registration<T> {}

// SAFETY: `Registration` offers no interior mutability (no mutation through &self
// and no mutable access is exposed)
unsafe impl<T> Sync for Registration<T> {}