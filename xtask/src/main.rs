use std::{
    env,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use structopt::StructOpt;
use anyhow::anyhow;
use duct::cmd;
//use common;

#[derive(StructOpt, Debug)]
#[structopt(name = "xtask")]
struct MainOpt {
    /// Example: thumbv7m-none-eabi
    #[structopt(short = "t", long = "target", default_value = "thumbv7em-none-eabihf")]
    target_tripple: String,

    /// Program name to run on the device (found in target/<target>/<profile>/)
    #[structopt(short = "n", long = "program-name", default_value = "hello") ]
    program_name: String,

    /// OpenOCD Interface (jlink, stlink, ... found here: /usr/share/openocd/scripts/interface/)
    #[structopt(short = "i", long = "openocd-interface", default_value = "jlink")]
    openocd_interface: String,

    /// OpenOCD Target (nrf52, stm32f1x, ... found here: /usr/share/openocd/scripts/target/ )
    #[structopt(long = "openocd-target", default_value = "nrf52")]
    openocd_target: String,

    /// OpenOCD Transport (swd, hla_swd) - nrf52 requires swd, stm32f103 requires hla_swd
    #[structopt(long = "openocd-transport", default_value = "swd")]
    openocd_transport: String,

    /// RTT TCP Port (8765)
    #[structopt(short = "p", long = "rtt-tcp-port", default_value = "8765")]
    rtt_tcp_port: u16,
}

fn main() -> Result<(), anyhow::Error> {
    let main_opt = MainOpt::from_args();
    println!("{:#?}", main_opt);

    env::set_current_dir(repo_root()?)?;

    gdb(main_opt)
}

fn repo_root() -> Result<PathBuf, anyhow::Error> {
    // Ensure our CWD is the root of the workspace. (in case we ran this from the xtask package directory)
    Ok(PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        // from there go one level up
        .parent()
        .unwrap()
        .to_owned())
}

fn gdb(main_opt: MainOpt) -> Result<(), anyhow::Error> {
    const BP_LENGTH: u8 = 2; // breakpoint length
    const RTT_BLOCK_IF_FULL: u32 = 2; // bit in `flags` field
    const RTT_FLAGS: u32 = 44; // offset of `flags` field in control block
    const RTT_ID: &str = "SEGGER RTT"; // control block ID
    const RTT_SIZE: u8 = 48; // control block size
    const THUMB_BIT: u32 = 1;

    //cmd!("cargo", "build", "--target", CARGO_TARGET).run()?;
    cmd!("cargo", "build").run()?;

    let elf = Path::new("target")
        .join(main_opt.target_tripple)
        .join("debug")
        .join(main_opt.program_name);

    // get symbol addresses from ELF
    let nm = cmd!("nm", "-C", &elf).read()?;
    let mut rtt = None;
    let mut main = None;
    for line in nm.lines() {
        if line.ends_with("_SEGGER_RTT") {
            rtt = line.splitn(2, ' ').next();
        } else if line.ends_with("main") {
            main = line.splitn(2, ' ').next();
        }
    }

    let rtt = u32::from_str_radix(
        rtt.ok_or_else(|| anyhow!("RTT control block not found"))?,
        16,
    )?;
    let main = u32::from_str_radix(
        main.ok_or_else(|| anyhow!("`main` function not found"))?,
        16,
    )? & !THUMB_BIT;

    #[rustfmt::skip]
    let openocd = cmd!(
        "openocd",
        "-f", "openocd.cfg",
        "-d0",
        "-c", format!("source [find interface/{}.cfg]", main_opt.openocd_interface),
        "-c", format!("transport select {}", main_opt.openocd_transport),
        "-c", format!("source [find target/{}.cfg]", main_opt.openocd_target),
        "-c", "init",
        "-c", format!("rtt server start {} 0", main_opt.rtt_tcp_port),
        "-c", "reset init",
        "-c", format!("flash write_image erase {}", elf.display()),
        "-c", "reset halt",
        "-c", format!("rtt setup {} {} {:?}", rtt, RTT_SIZE, RTT_ID),
        "-c", format!("bp {} {} hw", main, BP_LENGTH),
        "-c", "resume",
        "-c", format!("mww {} {}", rtt + RTT_FLAGS, RTT_BLOCK_IF_FULL),
        "-c", "rtt start",
    )
    .stderr_to_stdout()
    .reader()?;

    let mut lines = BufReader::new(openocd).lines();

    while let Some(line) = lines.next() {
        let line = line?;
        println!("{}", line);

        if line.contains("wrote") {
            break;
        }
    }

    cmd!("nc", "localhost", main_opt.rtt_tcp_port.to_string())
        .pipe(cmd!("defmt-print", "-e", &elf))
        .run()?;

    // close `openocd` *after* `nc`
    drop(lines);

    Ok(())
}
