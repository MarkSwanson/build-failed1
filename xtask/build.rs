use vergen::{Config, vergen};

fn main() -> Result<(), ()> {
    let config = Config::default();

    vergen(config).unwrap();

    Ok(())
}

