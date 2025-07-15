//! Módulo com os tipos e funções necessárias para o cadastro de um aluno novo.
use crate::ldap::{Cadastro, CadastroErro, consultar_cadastro_ldap, cadastrar_usuario};
use crate::portal_ufrj::{Consulta, ConsultaErro, consulta};
use crate::utils::nome::Nome;
use crate::utils::validacao_entradas::*;
use crate::configuracao::ConfiguracaoUsuario;
use secrecy::SecretString;
use serde::Deserialize;
use thiserror::Error;

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
    pub data: String,
    /// A hora de emissão contida no documento, no formato `hh:mm`, 24 horas.
    pub hora: String,
    /// O código contido no documento, no formato
    /// `XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX`.
    pub codigo: String,

    /// O nome completo. Não pode variar muito do documento do SIGA, mudanças
    /// pequenas como a existência ou não de um "de" e adição de acentos são
    /// válidas, mas qualquer mudança nas letras não é.
    ///
    /// Para fazer essa verificação, é usado o [`Nome`], mais informações sobre
    /// essa comparação podem ser encontrados na documentação desse tipo.
    pub nome: String,
    /// O email externo. Precisa ser um email válido.
    pub email: String,
    /// O telefone. Precisa ser um telefone válido.
    pub telefone: String,
    /// A senha. Precisa ter entre 6 e 32 caracteres.
    pub senha: SecretString,
}

#[derive(Debug, Error)]
pub enum ErroDeCadastro {
    #[error("o DRE {0} não é válido")]
    DREInvalido(String),
    #[error("a data {0} não é válido")]
    DataInvalida(String),
    #[error("a hora {0} não é válido")]
    HoraInvalida(String),
    #[error("o código {0} não é válido")]
    CodigoInvalido(String),
    #[error("o nome {0} não é válido")]
    NomeInvalido(String),
    #[error("o email {0} não é válido")]
    EmailInvalido(String),
    #[error("o telefone {0} não é válido")]
    TelefoneInvalido(String),
    #[error("a senha precisa ter entre 6 e 32 caracteres")]
    SenhaInvalida,

    #[error("não foi possível obter informações do SIGA")]
    ErroNaConsulta(#[from] ConsultaErro),
    #[error("alunos de {0} não têm direito à conta do IC")]
    AlunoOutroCurso(String),
    #[error("seu documento de matrícula é inválido")]
    DocumentoInvalido,

    #[error("houve um problema ao verificar o estado do cadastro no LDAP")]
    ErroNoCadastro(#[from] CadastroErro),
    #[error("o cadastro já existe, com o username {0}")]
    CadastroRedundante(String),

    #[error("O nome informado {informado} não é o mesmo do SIGA {siga}")]
    NomesDiferentes { informado: String, siga: String },
}

impl DadosParaCadastro {
    pub async fn cadastrar_sem_verificar_documento(
        mut self,
        uid: String,
        config: &ConfiguracaoUsuario,
        ldap_url: &str,
        ldap_bind_dn: &str,
        ldap_bind_pw: &str,
    ) -> Result<(), ErroDeCadastro> {
        self.dre = processar_dre(&self.dre)
            .ok_or_else(move || ErroDeCadastro::DREInvalido(self.dre))?;
        self.nome = processar_nome(&self.nome)
            .ok_or_else(move || ErroDeCadastro::NomeInvalido(self.nome))?;
        self.email = processar_email(&self.email)
            .ok_or_else(move || ErroDeCadastro::EmailInvalido(self.email))?;
        self.telefone =
            processar_telefone(&self.telefone).ok_or_else(move || {
                ErroDeCadastro::TelefoneInvalido(self.telefone)
            })?;
        validar_senha(&self.senha)
            .then_some(())
            .ok_or(ErroDeCadastro::SenhaInvalida)?;

        cadastrar_usuario(
            Cadastro::CadastroDisponivel(uid),
            &self,
            config,
            ldap_url,
            ldap_bind_dn,
            ldap_bind_pw,
        ).await?;

        Ok(())
    }

    pub async fn cadastrar(
        mut self,
        config: &ConfiguracaoUsuario,
        ldap_url: &str,
        ldap_bind_dn: &str,
        ldap_bind_pw: &str,
    ) -> Result<String, ErroDeCadastro> {
        self.data = processar_data(&self.data)
            .ok_or_else(move || ErroDeCadastro::DataInvalida(self.data))?;
        self.hora = processar_hora(&self.hora)
            .ok_or_else(move || ErroDeCadastro::HoraInvalida(self.hora))?;
        self.codigo = processar_codigo(&self.codigo)
            .ok_or_else(move || ErroDeCadastro::CodigoInvalido(self.codigo))?;

        // Faz a consulta no SIGA e no LDAP ao mesmo tempo
        let (consulta_siga, consulta_ldap) = tokio::join!(
            consulta(&self.dre, &self.data, &self.hora, &self.codigo),
            consultar_cadastro_ldap(
                &self.dre,
                &self.nome,
                ldap_url,
                ldap_bind_dn,
                ldap_bind_pw
            ),
        );

        let uid_ldap = match consulta_ldap? {
            Cadastro::CadastroDisponivel(uid) => uid,
            Cadastro::CadastroRedundante(uid) => {
                Err(ErroDeCadastro::CadastroRedundante(uid))?
            },
        };

        let nome_siga = match consulta_siga? {
            Consulta::AlunoDoCurso { nome } => nome,
            Consulta::AlunoOutroCurso { curso, .. } => {
                Err(ErroDeCadastro::AlunoOutroCurso(curso))?
            },
            Consulta::Desconhecido => Err(ErroDeCadastro::DocumentoInvalido)?,
        };

        // Verifica se o nome é o mesmo do SIGA
        if self.nome.parse::<Nome>() != nome_siga.parse() {
            Err(ErroDeCadastro::NomesDiferentes {
                informado: self.nome.clone(),
                siga: nome_siga,
            })?
        }

        self.cadastrar_sem_verificar_documento(
            uid_ldap.clone(),
            config,
            ldap_url,
            ldap_bind_dn,
            ldap_bind_pw,
        ).await?;

        Ok(uid_ldap)
    }
}
