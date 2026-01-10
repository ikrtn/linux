// SPDX-License-Identifier: GPL-2.0

//! I2C subsystem

// I2C Algorithm abstractions.
use crate::{
    bindings::{
        i2c_adapter,
        i2c_msg,
        i2c_smbus_data,
        u16_,
        u32_,
        u8_, //
    },
    device::Bound,
    error::VTABLE_DEFAULT_ERROR, //
    i2c::adapter::I2cAdapter,
    prelude::*,
    types::Opaque,
};

use core::{
    marker::PhantomData,
};

/// The i2c msg representation.
///
/// This structure represents the Rust abstraction for a C `struct i2c_msg`. The
/// implementation abstracts the usage of an existing C `struct i2c_msg` that
/// gets passed from/to the C side
///
/// # Invariants
///
/// A [`I2cMsg`] instance represents a valid `struct i2c_msg` created by the C portion of
/// the kernel.
#[repr(transparent)]
pub struct I2cMsg(Opaque<bindings::i2c_msg>);

impl I2cMsg {
    /// Convert a raw C `struct i2c_msg` pointers to `&'a mut [I2cMsg]`.
    pub fn from_raw_parts_mut<'a>(msgs: *mut bindings::i2c_msg, len: usize) -> &'a mut [Self] {
        // SAFETY: Callers must ensure that `msgs` is valid, non-null, for the duration of this 
        // function call and the entire duration when the returned slice exists.
        unsafe { core::slice::from_raw_parts_mut(msgs.cast::<Self>(), len) }
    }
}

/// The i2c smbus data representation.
///
/// This structure represents the Rust abstraction for a C `struct i2c_smbus_data`. The
/// implementation abstracts the usage of an existing C `struct i2c_smbus_data` that
/// gets passed from/to the C side
///
/// # Invariants
///
/// A [`I2cSmbusData`] instance represents a valid `struct i2c_msg` created by the C portion of
/// the kernel.
#[repr(transparent)]
pub struct I2cSmbusData(Opaque<bindings::i2c_smbus_data>);

impl I2cSmbusData {
    /// Convert a raw C `struct i2c_smbus_data` pointer to `&'a I2cSmbusData`.
    fn from_raw<'a>(ptr: *const bindings::i2c_smbus_data) -> &'a Self {
        // SAFETY: Callers must ensure that `ptr` is valid, non-null, for the duration of this 
        // function call and the entire duration when the returned reference exists.
        unsafe { &*ptr.cast() }
    }
}

/// Trait implemented by the private data of an i2c adapter.
#[vtable]
pub trait I2cAlgorithm {
    /// Handler for transfer a given number of messages defined by the msgs array 
    /// via the specified adapter.
    fn xfer(_adap: &I2cAdapter<Bound>, _msgs: &mut [I2cMsg]) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// Same as @xfer. Yet, only using atomic context so e.g. PMICs
    /// can be accessed very late before shutdown. Optional.
    fn xfer_atomic(_adap: &I2cAdapter<Bound>, _msgs: &mut [I2cMsg]) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// Issue SMBus transactions to the given I2C adapter. If this
    /// is not present, then the bus layer will try and convert the SMBus calls
    /// into I2C transfers instead.
    fn smbus_xfer(
        _adap: &I2cAdapter<Bound>,
        _addr: u16,
        _flags: u16,
        _read_write: u8,
        _command: u8,
        _size: usize,
        _data: &I2cSmbusData,
    ) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// Same as @smbus_xfer. Yet, only using atomic context
    /// so e.g. PMICs can be accessed very late before shutdown. Optional.
    fn smbus_xfer_atomic(
        _adap: &I2cAdapter<Bound>,
        _addr: u16,
        _flags: u16,
        _read_write: u8,
        _command: u8,
        _size: usize,
        _data: &I2cSmbusData,
    ) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    /// Return the flags that this algorithm/adapter pair supports
    /// from the ``I2C_FUNC_*`` flags.
    fn functionality(_adap: &I2cAdapter<Bound>) -> u32 {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}

/// A vtable for the I2C xfer operations of a Rust i2c adapter.
pub struct I2cAlgorithmVTable<T: I2cAlgorithm>(PhantomData<T>);

impl<T: I2cAlgorithm> I2cAlgorithmVTable<T> {
    /// # Safety
    ///
    /// `adap` must be a valid pointer to `struct i2c_adapter` that is associated with a
    /// `I2cAdapterRegistration<T>`. 
    /// `msgs` must be a valid pointer to `struct i2c_msg` for reading/writing.
    unsafe extern "C" fn xfer(
        adap: *mut i2c_adapter,
        msgs: *mut i2c_msg,
        num: ffi::c_int,
    ) -> ffi::c_int {
        let num = match usize::try_from(num) {
            Ok(num) => num,
            Err(_err) => return EINVAL.to_errno(),
        };

        let msg_slice = I2cMsg::from_raw_parts_mut(msgs, num);

        match T::xfer(I2cAdapter::from_raw(adap), msg_slice) {
            Ok(()) => 0,
            Err(err) => err.to_errno(),
        }
    }

    /// # Safety
    ///
    /// `adap` must be a valid pointer to `struct i2c_adapter` that is associated with a
    /// `I2cAdapterRegistration<T>`. 
    /// `msgs` must be a valid pointer to `struct i2c_msg` for reading/writing.
    unsafe extern "C" fn xfer_atomic(
        adap: *mut i2c_adapter,
        msgs: *mut i2c_msg,
        num: ffi::c_int,
    ) -> ffi::c_int {
        let num = match usize::try_from(num) {
            Ok(num) => num,
            Err(_err) => return EINVAL.to_errno(),
        };

        let msg_slice = I2cMsg::from_raw_parts_mut(msgs, num);

        match T::xfer_atomic(I2cAdapter::from_raw(adap), msg_slice) {
            Ok(()) => 0,
            Err(err) => err.to_errno(),
        }
    }

    /// # Safety
    ///
    /// `adap` must be a valid pointer to `struct i2c_adapter` that is associated with a
    /// `I2cAdapterRegistration<T>`. 
    /// `data` must be a valid pointer to `struct i2c_smbus_data` for reading/writing.
    unsafe extern "C" fn smbus_xfer(
        adap: *mut i2c_adapter,
        addr: u16_,
        flags: ffi::c_ushort,
        read_write: ffi::c_char,
        command: u8_,
        size: ffi::c_int,
        data: *mut i2c_smbus_data,
    ) -> ffi::c_int {
        let size = match usize::try_from(size) {
            Ok(size) => size,
            Err(_err) => return EINVAL.to_errno(),
        };

        let data = I2cSmbusData::from_raw(data);

        match T::smbus_xfer(
            I2cAdapter::from_raw(adap),
            addr,
            flags,
            read_write,
            command,
            size,
            data,
        ) {
            Ok(()) => 0,
            Err(err) => err.to_errno(),
        }
    }

    /// # Safety
    ///
    /// `adap` must be a valid pointer to `struct i2c_adapter` that is associated with a
    /// `I2cAdapterRegistration<T>`. 
    /// `data` must be a valid pointer to `struct i2c_smbus_data` for reading/writing.
    unsafe extern "C" fn smbus_xfer_atomic(
        adap: *mut i2c_adapter,
        addr: u16_,
        flags: ffi::c_ushort,
        read_write: ffi::c_char,
        command: u8_,
        size: ffi::c_int,
        data: *mut i2c_smbus_data,
    ) -> ffi::c_int {
        let size = match usize::try_from(size) {
            Ok(size) => size,
            Err(_err) => return EINVAL.to_errno(),
        };

        let data = I2cSmbusData::from_raw(data);

        match T::smbus_xfer_atomic(
            I2cAdapter::from_raw(adap),
            addr,
            flags,
            read_write,
            command,
            size,
            data,
        ) {
            Ok(()) => 0,
            Err(err) => err.to_errno(),
        }
    }

    /// # Safety
    ///
    /// `adap` must be a valid pointer to `struct i2c_adapter` that is associated with a
    /// `I2cAdapterRegistration<T>`. 
    unsafe extern "C" fn functionality(adap: *mut i2c_adapter) -> u32_ {
        T::functionality(I2cAdapter::from_raw(adap))
    }

    const VTABLE: bindings::i2c_algorithm = bindings::i2c_algorithm {
        xfer: if T::HAS_XFER {
            Some(Self::xfer)
        } else {
            None
        },
        xfer_atomic: if T::HAS_XFER_ATOMIC {
            Some(Self::xfer_atomic)
        } else {
            None
        },
        smbus_xfer: if T::HAS_SMBUS_XFER {
            Some(Self::smbus_xfer)
        } else {
            None
        },
        smbus_xfer_atomic: if T::HAS_SMBUS_XFER_ATOMIC {
            Some(Self::smbus_xfer_atomic)
        } else {
            None
        },
        functionality: if T::HAS_FUNCTIONALITY {
            Some(Self::functionality)
        } else {
            None
        },
    };

    pub(super) const fn build() -> &'static bindings::i2c_algorithm {
        &Self::VTABLE
    }
}
