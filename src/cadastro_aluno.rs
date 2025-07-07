use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_email::Email;

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
