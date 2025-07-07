use std::error::Error;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    comando: Comandos,
}

#[derive(Subcommand)]
enum Comandos {
    Matricula {
        dre: String,
        data: String,
        hora: String,
        codigo: String,
    },
    Registro {
        dre: String,
        username: String,
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.comando {
        Comandos::Matricula { dre, data, hora, codigo } => {
            let r = alumnic::portal_ufrj::consulta(dre, data, hora, codigo)
                .await?;
            println!("{r:?}");
        }
        Comandos::Registro { dre, username } => {
            let r = alumnic::ldap::achar_usuario(dre, username).await?;
            println!("{r:?}");
        }
    }

    Ok(())
}

