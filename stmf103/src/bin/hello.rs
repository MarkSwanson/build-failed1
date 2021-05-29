#![no_main]
#![no_std]

use stmf103 as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    stmf103::exit()
}
   