//! Binary entry point
use rust_modules_fixture::Config;

fn main() {
    let config = Config::default();
    println!("Config: {:?}", config);
}
