use alumnic::configuracao::Configuracao;
use alumnic::cadastro_aluno::DadosParaCadastro;
use clap::{Parser, Subcommand};
use std::error::Error;
use dialoguer::{theme::ColorfulTheme, Password};
use secrecy::SecretString;

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
    NovoAluno {
        username: String,
        dre: String,
        nome: String,
        email: String,
        telefone: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let cfg = Configuracao::importar()?;

    match cli.comando {
        Comandos::Matricula {
            dre,
            data,
            hora,
            codigo,
        } => {
            let r =
                alumnic::portal_ufrj::consulta(&dre, &data, &hora, &codigo).await?;
            println!("{r:?}");
        },
        Comandos::Registro { dre, nome } => {
            let r = alumnic::ldap::consultar_cadastro_ldap(
                &dre,
                &nome,
                &cfg.ldap_url,
                &cfg.ldap_bind_dn,
                &cfg.ldap_bind_pw,
            )
            .await?;
            println!("{r:?}");
        },
        Comandos::NovoAluno { username, dre, nome, email, telefone } => {
            let senha: SecretString = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Senha")
                .with_confirmation("Confirmar senha", "Senhas diferentes")
                .interact()
                .unwrap()
                .into();

            let dados = DadosParaCadastro {
                dre,
                data: "".to_string(),
                hora: "".to_string(),
                codigo: "".to_string(),
                nome,
                email,
                telefone,
                senha,
            };

            dados.cadastrar_sem_verificar_documento(
                username,
                &cfg.usuario_novo,
                &cfg.ldap_url,
                &cfg.ldap_bind_dn,
                &cfg.ldap_bind_pw,
            ).await?;
        }
    }

    Ok(())
}
