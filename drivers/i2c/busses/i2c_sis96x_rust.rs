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

struct Sis96xRegs;

impl Sis96xRegs {
    const SMB_STS: usize = 0x00;
    const SMB_EN: usize = 0x01;
    const SMB_CNT: usize = 0x02;
    const SMB_HOST_CNT: usize = 0x03;
    const SMB_ADDR: usize = 0x04;
    const SMB_CMD: usize = 0x05;
    const SMB_PCOUNT: usize = 0x06;
    const SMB_COUNT: usize = 0x07;
    const SMB_BYTE: usize = 0x08;
    const SMB_DEV_ADDR: usize = 0x10;
    const SMB_DB0: usize = 0x11;
    const SMB_DB1: usize = 0x12;
    const SMB_SAA: usize = 0x13;
    const END: usize = 0x14;
}

type Bar0 = pci::Bar<{ Sis96xRegs::END }>;

/* SiS96x SMBus registers */

#[pin_data(PinnedDrop)]
struct Sis96xDriver {
    parent_dev: ARef<pci::Device>,
    #[pin]
    i2cAdap: Devres<Registration<Sis96xDevice>>,
    #[pin]
    bar: Devres<Bar0>,
}

struct Sis96xDevice {}

impl I2cAlgorithm for Sis96xDevice {

}

kernel::pci_device_table!(
    SIS96X_PCI_TABLE,
    MODULE_SIS96X_PCI_TABLE,
    <Sis96xDriver as pci::Driver>::IdInfo,
    [(pci::DeviceId::from_id(pci::Vendor::SI, bindings::PCI_DEVICE_ID_SI_SMBUS),())]
);

impl pci::Driver for Sis96xDriver {
    type IdInfo = ();

    const ID_TABLE: pci::IdTable<Self::IdInfo> = &SIS96X_PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _id_info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            pdev.set_master();

            Ok(try_pin_init!(Self {
                bar <- pdev.iomap_region_sized::<{ Sis96xRegs::END }>(0, c_str!("rust_driver_pci")),
                parent_dev: pdev.into(),
                i2cAdap <- {
                    let name = I2cAdapterOptions{ name: c_str!("i2c_sis96x_rust")};
                    Registration::register(pdev.as_ref(), name)
                },
            }))
        })
    }
}

#[pinned_drop]
impl PinnedDrop for Sis96xDriver {
    fn drop(self: Pin<&mut Self>) {
        dev_dbg!(self.parent_dev.as_ref(), "Remove Rust PCI driver sample.\n");
    }
}

kernel::module_pci_driver! {
    type: Sis96xDriver,
    name: "sis96x_smbus",
    authors: ["Igor Korotin"],
    description: "SiS96x SMBus driver",
    license: "GPL",
}