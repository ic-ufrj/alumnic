use chrono::{DateTime, Local};
use serde_email::Email;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DadosParaCadastro {
    pub dre: String,
    pub emissao: DateTime<Local>,
    pub codigo: String,
    pub email_externo: Email,
    // TODO: serde para telefones
    pub telefone: String,
    pub senha: String,
}


