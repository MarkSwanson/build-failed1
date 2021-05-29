#![no_main]
#![no_std]

use defmt_gdb as _; // global logger + panicking-behavior + memory layout

// https://ferrous-systems.com/blog/gdb-and-defmt/

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!(r#"Hello, world11!"#);

    defmt_gdb::exit()
}
