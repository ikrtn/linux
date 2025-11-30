#![allow(dead_code)]
#![allow(unreachable_pub)]
#![allow(unused_variables)]

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

pub struct I2cAdapterOptions{
    /// The name of the miscdevice.
    pub name: &'static CStr,
}

impl I2cAdapterOptions {
    pub const fn from_raw<T: I2cAlgorithm>(self) -> bindings::i2c_adapter {
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

// SAFETY: A `Registration` of a `struct i2c_client` can be released from any thread.
unsafe impl<T> Send for Registration<T> {}

// SAFETY: `Registration` offers no interior mutability (no mutation through &self
// and no mutable access is exposed)
unsafe impl<T> Sync for Registration<T> {}