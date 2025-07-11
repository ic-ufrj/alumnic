//! Módulo com os tipos e funções necessárias para o cadastro de um aluno novo.
use secrecy::SecretString;
use serde::Deserialize;
use serde_email::Email;

/// Struct contendo os dados para cadastrar um novo usuário. Esses dados são
/// recebidos pela aplicação e são o suficiente para cadastrar a maior parte
/// dos alunos, as exceções devem ser tratadas pela Supervisão. `dre`,
/// `data_emissao`, `hora_emissao` e `codigo` são dados contidos no documento
/// "Regularmente Matriculado" disponível no SIGA, que é autenticado pelo
/// programa. O `nome` deve ser o mesmo do SIGA.
#[derive(Deserialize)]
pub struct DadosParaCadastro {
    /// O DRE, somente números, 9 dígitos.
    pub dre: String,
    /// A data de emissão contida no documento, no formato `dd/mm/aaaa`.
    pub data_emissao: String,
    /// A hora de emissão contida no documento, no formato `hh:mm`, 24 horas.
    pub hora_emissao: String,
    /// O código contido no documento, no formato
    /// `XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX`.
    pub codigo: String,

    /// O nome completo. Não pode variar muito do documento do SIGA, mudanças
    /// pequenas como a existência ou não de um "de" e adição de acentos são
    /// válidas, mas qualquer mudança nas letras não é.
    ///
    /// Para fazer essa verificação, é usado o
    /// [`Nome`](crate::utils::nome::Nome), mais informações sobre essa
    /// comparação podem ser encontrados na documentação desse tipo.
    pub nome: String,
    /// O email externo. Precisa ser um email válido.
    pub email_externo: Email,
    /// O telefone. Precisa ser um telefone válido.
    pub telefone: String,
    /// A senha. Precisam ter entre 6 e 32 caracteres.
    pub senha: SecretString,
}
