use clap::{Parser, Subcommand};
use std::error::Error;

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
        nome: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.comando {
        Comandos::Matricula {
            dre,
            data,
            hora,
            codigo,
        } => {
            let r =
                alumnic::portal_ufrj::consulta(dre, data, hora, codigo).await?;
            println!("{r:?}");
        },
        Comandos::Registro { dre, nome } => {
            let r = alumnic::ldap::consultar_cadastro_ldap(dre, nome).await?;
            println!("{r:?}");
        },
    }

    Ok(())
}
