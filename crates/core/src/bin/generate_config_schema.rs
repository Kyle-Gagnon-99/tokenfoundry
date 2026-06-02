use std::error::Error;

use schemars::schema_for;
use tokenfoundry_core::config::Config;

fn main() -> Result<(), Box<dyn Error>> {
    let schema = schema_for!(Config);
    serde_json::to_writer_pretty(std::io::stdout(), &schema)?;
    println!();
    Ok(())
}
