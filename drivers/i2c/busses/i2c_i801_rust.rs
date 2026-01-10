#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(unused_variables)]

use kernel::{
    bindings,
    c_str,
    device::{Bound, Core},
    devres::Devres,
    i2c::adapter::{
        I2cAdapter, //
        I2cAdapterOptions,
        Registration,
    },
    i2c::algo::{
        I2cAlgorithm,
        I2cSmbusData, //
    },
    pci,
    prelude::*,
    sync::aref::ARef, //
};

struct I801IoRegs;

impl I801IoRegs {
    const SMBHSTSTS: usize = 0x000;
    const SMBHSTCNT: usize = 0x002;
    const SMBHSTCMD: usize = 0x003;
    const SMBHSTADD: usize = 0x004;
    const SMBHSTDAT0: usize = 0x005;
    const SMBHSTDAT1: usize = 0x006;
    const SMBBLKDAT: usize = 0x007;
    const SMBPEC: usize = 0x008;	/* ICH3 and later */
    const SMBAUXSTS: usize = 0x00C;	/* ICH4 and later */
    const SMBAUXCTL: usize = 0x00D;	/* ICH4 and later */
    const SMBSLVSTS: usize = 0x010;	/* ICH3 and later */
    const SMBSLVCMD: usize = 0x011;	/* ICH3 and later */
    const SMBNTFDADD: usize = 0x014;	/* ICH3 and later */
    const END: usize = 0x015;
}

struct I801Regs;

impl I801Regs {
    const SMBHSTCFG: usize = 0x040;
    const TCOBASE: usize = 0x050;
    const TCOCTL: usize = 0x054;
    const END: usize = 0x058;
}

const SMBBAR_MMIO: u32 = 0;
const SMBBAR: u32 = 4;

type I801IoBar = pci::Bar<{ I801IoRegs::END }>;
type I801Bar = pci::Bar<{ I801Regs::END }>;

/* I801 SMBus registers */

#[pin_data(PinnedDrop)]
struct I801Driver {
    parent_dev: ARef<pci::Device>,
    #[pin]
    i2c_adap: Devres<Registration<I801Device>>,
    #[pin]
    bar0: Devres<I801IoBar>,
    #[pin]
    bar4: Devres<I801Bar>,
}

struct I801Device {}

#[vtable]
impl I2cAlgorithm for I801Device {
    fn smbus_xfer(
        adap: &I2cAdapter<Bound>,
        addr: u16,
        flags: u16,
        read_write: u8,
        command: u8,
        size: usize,
        data: &I2cSmbusData,
    ) -> Result {
        Ok(())
    }

    fn functionality(adap: &I2cAdapter<Bound>) -> u32 {
        0
    }
}

kernel::pci_device_table!(
    I801_PCI_TABLE,
    MODULE_I801_PCI_TABLE,
    <I801Driver as pci::Driver>::IdInfo,
    [(
        pci::DeviceId::from_id(pci::Vendor::INTEL, bindings::PCI_DEVICE_ID_INTEL_ICH9_6),
        ()
    )]
);

impl pci::Driver for I801Driver {
    type IdInfo = ();

    const ID_TABLE: pci::IdTable<Self::IdInfo> = &I801_PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _id_info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            pdev.set_master();

            Ok(try_pin_init!(Self {
                bar0 <- pdev.iomap_region_sized::<{ I801IoRegs::END }>(SMBBAR_MMIO, c_str!("i801_smbus/bar0")),
                bar4 <- pdev.iomap_region_sized::<{ I801Regs::END }>(SMBBAR, c_str!("i801_smbus/bar4")),
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
