use std::error::Error;
use alumnic::ldap::achar_usuario;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "{:?}",
        achar_usuario("dre", "uid").await?,
    );

    Ok(())
}

