//! Módulo com os tipos e funções necessárias para o cadastro de um aluno novo.
use crate::configuracao::ConfiguracaoUsuario;
use crate::ldap::ErroLdap;
use crate::ldap::cadastrar::cadastrar_usuario;
use crate::ldap::consulta::{
    Consulta as ConsultaLdap, consultar_cadastro_ldap,
};
use crate::portal_ufrj::{Consulta, ConsultaErro, consulta};
use crate::utils::nome::Nome;
use crate::utils::validacao_entradas::*;
use axum::http::StatusCode;
use secrecy::SecretString;
use serde::Deserialize;
use thiserror::Error;

/// Struct contendo os dados para cadastrar um novo usuário. Esses dados são
/// recebidos pela aplicação e são o suficiente para cadastrar a maior parte
/// dos alunos, as exceções devem ser tratadas pela Supervisão. `dre`,
/// `data_emissao`, `hora_emissao` e `codigo` são dados contidos no documento
/// "Regularmente Matriculado" disponível no SIGA, que é autenticado pelo
/// programa. O `nome` deve ser o mesmo do SIGA.
#[derive(Debug, Deserialize)]
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
    /// A senha. Precisa ter entre 8 e 25 caracteres, ao menos uma letra
    /// minúscula, maiúscula e um dígito.
    pub senha: SecretString,
}

#[derive(Debug, Error)]
pub enum ErroDeCadastro {
    #[error("O DRE {0:?} não é válido")]
    DREInvalido(String),
    #[error("A data {0:?} não é válida")]
    DataInvalida(String),
    #[error("A hora {0:?} não é válida")]
    HoraInvalida(String),
    #[error("O código {0:?} não é válido")]
    CodigoInvalido(String),
    #[error("O nome {0:?} não é válido")]
    NomeInvalido(String),
    #[error("O email {0:?} não é válido")]
    EmailInvalido(String),
    #[error("O telefone {0:?} não é válido")]
    TelefoneInvalido(String),
    // TODO: mudar verificacao da senha
    #[error("A senha precisa ter entre 8 e 25 caracteres, uma letra minúscula, uma maiúscula e um dígito")]
    SenhaInvalida,

    #[error("Não foi possível obter informações do SIGA")]
    ErroNaConsulta(#[from] ConsultaErro),
    #[error("Alunos de {0} não têm direito à conta do IC")]
    AlunoOutroCurso(String),
    #[error("Seu documento de matrícula é inválido")]
    DocumentoInvalido,

    #[error("Houve um problema ao verificar o estado do cadastro no LDAP")]
    ErroNoCadastro(#[from] ErroLdap),
    #[error("O cadastro já existe, com o username {0:?}")]
    CadastroRedundante(String),

    #[error("O nome informado {informado:?} não é o mesmo do SIGA {siga:?}")]
    NomesDiferentes { informado: String, siga: String },
}

impl ErroDeCadastro {
    pub fn status(&self) -> StatusCode {
        match self {
            ErroDeCadastro::DREInvalido(..)
            | ErroDeCadastro::DataInvalida(..)
            | ErroDeCadastro::HoraInvalida(..)
            | ErroDeCadastro::CodigoInvalido(..)
            | ErroDeCadastro::NomeInvalido(..)
            | ErroDeCadastro::EmailInvalido(..)
            | ErroDeCadastro::TelefoneInvalido(..)
            | ErroDeCadastro::SenhaInvalida
            | ErroDeCadastro::NomesDiferentes { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            },
            ErroDeCadastro::AlunoOutroCurso(..)
            | ErroDeCadastro::DocumentoInvalido => StatusCode::FORBIDDEN,
            ErroDeCadastro::ErroNaConsulta(..)
            | ErroDeCadastro::ErroNoCadastro(..) => {
                StatusCode::INTERNAL_SERVER_ERROR
            },
            ErroDeCadastro::CadastroRedundante(..) => StatusCode::CONFLICT,
        }
    }
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
            uid,
            &self,
            config,
            ldap_url,
            ldap_bind_dn,
            ldap_bind_pw,
        )
        .await?;

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
            ConsultaLdap::CadastroDisponivel(uid) => uid,
            ConsultaLdap::CadastroRedundante(uid) => {
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
        )
        .await?;

        Ok(uid_ldap)
    }
}
