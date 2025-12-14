#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_snake_case)]

use kernel::{ 
    bindings, 
    c_str, 
    device::Core, 
    devres::Devres, 
    i2c::adapter::{
        Registration,
        I2cAdapterOptions,
    },
    i2c::algo::I2cAlgorithm,
    pci, 
    prelude::*, 
    sync::aref::ARef //
};

struct I801Regs;

impl I801Regs {
    const SMBHSTCFG: usize = 0x040;
    const TCOBASE: usize = 0x050;
    const TCOCTL: usize = 0x054;
    const END: usize = 0x058;
}

const SMBBAR: u32 = 4;

type I801Bar = pci::Bar<{ I801Regs::END }>;

/* I801 SMBus registers */

#[pin_data(PinnedDrop)]
struct I801Driver {
    parent_dev: ARef<pci::Device>,
    #[pin]
    i2c_adap: Devres<Registration<I801Device>>,
    #[pin]
    bar: Devres<I801Bar>,
}

struct I801Device {}

impl I2cAlgorithm for I801Device {

}

kernel::pci_device_table!(
    I801_PCI_TABLE,
    MODULE_I801_PCI_TABLE,
    <I801Driver as pci::Driver>::IdInfo,
    [(pci::DeviceId::from_id(pci::Vendor::INTEL, bindings::PCI_DEVICE_ID_INTEL_ICH9_6),())]
);

impl pci::Driver for I801Driver {
    type IdInfo = ();

    const ID_TABLE: pci::IdTable<Self::IdInfo> = &I801_PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _id_info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            pdev.set_master();

            Ok(try_pin_init!(Self {
                bar <- pdev.iomap_region_sized::<{ I801Regs::END }>(SMBBAR, c_str!("rust_driver_pci")),
                parent_dev: pdev.into(),
                i2c_adap <- {
                    let name = I2cAdapterOptions{ name: c_str!("i2c_i801_rust")};
                    Registration::register(pdev.as_ref(), name)
                },
            }))
        })
    }
}

#[pinned_drop]
impl PinnedDrop for I801Driver {
    fn drop(self: Pin<&mut Self>) {
        dev_dbg!(self.parent_dev.as_ref(), "Remove Rust PCI driver sample.\n");
    }
}

kernel::module_pci_driver! {
    type: I801Driver,
    name: "i801_smbus",
    authors: ["Igor Korotin"],
    description: "I801 SMBus driver",
    license: "GPL",
}