#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unreachable_pub)]
use crate::{
    bindings::{
        i2c_adapter, 
        i2c_msg,
        i2c_smbus_data,
        u32_,
        u16_,
        u8_, //
    }, 
    error::VTABLE_DEFAULT_ERROR, //
    i2c::I2cAdapter,
    prelude::*,
};

use core::{
    marker::PhantomData,
};

pub trait I2cAlgorithm {
    fn xfer(adap: &I2cAdapter) {
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
        0
    }

    unsafe extern "C" fn xfer_atomic(
           adap: *mut i2c_adapter,
           msgs: *mut i2c_msg,
           num: ffi::c_int,
        ) -> ffi::c_int {
        0
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
        0
    }
    
    unsafe extern "C" fn smbus_xfer_atomic(
            adap: *mut i2c_adapter,
            addr: u16_,
            flags: ffi::c_ushort,
            read_write: ffi::c_char,
            command: u8_,
            size: ffi::c_int,
            data: *mut i2c_smbus_data,
        ) -> ffi::c_int{
        0
    }
    unsafe extern "C" fn functionality(adap: *mut i2c_adapter) -> u32_ {
        0
    }

    const VTABLE: bindings::i2c_algorithm = bindings::i2c_algorithm {
        xfer: Some(Self::xfer),
        xfer_atomic: Some(Self::xfer_atomic),
        smbus_xfer: Some(Self::smbus_xfer),
        smbus_xfer_atomic: Some(Self::smbus_xfer_atomic),
        functionality: Some(Self::functionality),
    };

    pub const fn build() -> &'static bindings::i2c_algorithm {
        &Self::VTABLE
    }
}
