// SPDX-License-Identifier: GPL-2.0

//! Rust I2C adapter registration sample.
//!
//! An I2C adapter in Rust cannot exist on its own. To register a new I2C adapter,
//! it must be bound to a parent device. In this sample driver, a platform device
//! is used as the parent.

//! ACPI match table test
//!
//! This demonstrates how to test an ACPI-based Rust I2C adapter registration driver
//! using QEMU with a custom SSDT.
//!
//! Steps:
//!
//! 1. **Create an SSDT source file** (`ssdt.dsl`) with the following content:
//!
//!     ```asl
//!     DefinitionBlock ("", "SSDT", 2, "TEST", "VIRTACPI", 0x00000001)
//!     {
//!         Scope (\_SB)
//!         {
//!             Device (T432)
//!             {
//!                 Name (_HID, "LNUXBEEF")  // ACPI hardware ID to match
//!                 Name (_UID, 1)
//!                 Name (_STA, 0x0F)        // Device present, enabled
//!                 Name (_CRS, ResourceTemplate ()
//!                 {
//!                     Memory32Fixed (ReadWrite, 0xFED00000, 0x1000)
//!                 })
//!             }
//!         }
//!     }
//!     ```
//!
//! 2. **Compile the table**:
//!
//!     ```sh
//!     iasl -tc ssdt.dsl
//!     ```
//!
//!     This generates `ssdt.aml`
//!
//! 3. **Run QEMU** with the compiled AML file:
//!
//!     ```sh
//!     qemu-system-x86_64 -m 512M \
//!         -enable-kvm \
//!         -kernel path/to/bzImage \
//!         -append "root=/dev/sda console=ttyS0" \
//!         -hda rootfs.img \
//!         -serial stdio \
//!         -acpitable file=ssdt.aml
//!     ```
//!
//!     Requirements:
//!     - The `rust_i2c_adapter` must be present either:
//!         - built directly into the kernel (`bzImage`), or
//!         - available as a `.ko` file and loadable from `rootfs.img`
//!
//! 4. **Verify it worked** by checking `dmesg`:
//!
//!     ```
//!     rust_i2c_adapter LNUXBEEF:00: Probe Rust I2C Adapter registration sample.
//!     ```
//!

use kernel::{
    acpi,
    device,
    devres::Devres,
    i2c::adapter::{
        I2cAdapter,
        I2cAdapterOptions,
        Registration, //
    },
    i2c::algo::{
        flags, //
        I2cAlgorithm,
        I2cFlags,
        I2cSmbusData,
    },
    platform,
    prelude::*,
    sync::aref::ARef, //
};

#[pin_data]
struct SampleDriver {
    parent_dev: ARef<platform::Device>,
    #[pin]
    i2c_adap: Devres<Registration<SampleDevice>>,
}

struct SampleDevice {}

#[vtable]
impl I2cAlgorithm for SampleDevice {
    fn smbus_xfer(
        _adap: &I2cAdapter<device::Normal>,
        _addr: u16,
        _flags: u16,
        _read_write: u8,
        _command: u8,
        _size: usize,
        _data: &I2cSmbusData,
    ) -> Result {
        dev_info!(_adap.as_ref(), "SMBus xfer: request handler called");
        Ok(())
    }

    fn functionality(_adap: &I2cAdapter<device::Normal>) -> I2cFlags {
        flags::I2C_FUNC_SMBUS_READ_BYTE | flags::I2C_FUNC_SMBUS_WRITE_BYTE
    }
}

kernel::acpi_device_table!(
    ACPI_TABLE,
    MODULE_ACPI_TABLE,
    <SampleDriver as platform::Driver>::IdInfo,
    [(acpi::DeviceId::new(c"LNUXBEEF"), ())]
);

impl platform::Driver for SampleDriver {
    type IdInfo = ();

    const ACPI_ID_TABLE: Option<acpi::IdTable<Self::IdInfo>> = Some(&ACPI_TABLE);

    fn probe(
        pdev: &platform::Device<device::Core>,
        _id_info: Option<&Self::IdInfo>,
    ) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            dev_info!(
                pdev.as_ref(),
                "Probe Rust I2C Adapter registration sample.\n"
            );

            Ok(try_pin_init!(Self {
                parent_dev: pdev.into(),
                i2c_adap <- {
                    let name = I2cAdapterOptions{ name: c"rust_i2c_adapter"};
                    Registration::register(pdev.as_ref(), name)
                },
            }))
        })
    }

    fn unbind(pdev: &platform::Device<device::Core>, _this: Pin<&Self>) {
        dev_info!(
            pdev.as_ref(),
            "Unbind start: Rust I2C Adapter registration sample.\n"
        );
        // The i2c_adap (Devres<Registration<SampleDevice>>) will be automatically
        // dropped here, which will call i2c_del_adapter() in its Drop impl
        dev_info!(
            pdev.as_ref(),
            "Unbind complete: Rust I2C Adapter registration sample.\n"
        );
    }
}

kernel::module_platform_driver! {
    type: SampleDriver,
    name: "rust_i2c_adapter",
    authors: ["Igor Korotin"],
    description: "Sample I2C Adapter registration",
    license: "GPL",
}
