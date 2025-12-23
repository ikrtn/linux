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
    mem::MaybeUninit,
};

#[repr(transparent)]
pub struct I2cMsg(Opaque<bindings::i2c_msg>);

impl I2cMsg {
    pub fn from_raw_parts_mut<'a>(msgs: *mut bindings::i2c_msg, len: usize) -> &'a mut [Self] {
        unsafe { core::slice::from_raw_parts_mut(msgs as *mut Self, len) }
    }
}

#[repr(transparent)]
pub struct I2cSmbusData(Opaque<bindings::i2c_smbus_data>);

impl I2cSmbusData {
    unsafe fn from_raw<'a>(ptr: *const bindings::i2c_smbus_data) -> &'a Self {
        unsafe { &*ptr.cast() }
    }
}

pub trait I2cAlgorithm {
    const HAS_FUNCTIONALITY: bool = false;
    const HAS_SMBUS_XFER: bool = false;
    const HAS_SMBUS_XFER_ATOMIC: bool = false;
    const HAS_XFER: bool = false;
    const HAS_XFER_ATOMIC: bool = false;
    fn xfer(_adap: &I2cAdapter<Bound>, _msgs: &mut [I2cMsg]) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn xfer_atomic(_adap: &I2cAdapter<Bound>, _msgs: &mut [I2cMsg]) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

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

    fn functionality(_adap: &I2cAdapter<Bound>) -> u32 {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}

/// A vtable for the file operations of a Rust miscdevice.
pub struct I2cAlgorithmVTable<T: I2cAlgorithm>(PhantomData<T>);

impl<T: I2cAlgorithm> I2cAlgorithmVTable<T> {
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

        let data = unsafe { I2cSmbusData::from_raw(data) };

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

        let data = unsafe { I2cSmbusData::from_raw(data) };

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

    // SAFETY: The caller provides a valid `struct i2c_adapter` associated with a
    // `I2cAdapterRegistration<T>` file.
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
        // SAFETY: All zeros is a valid value for `bindings::file_operations`.
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    pub const fn build() -> &'static bindings::i2c_algorithm {
        &Self::VTABLE
    }
}
