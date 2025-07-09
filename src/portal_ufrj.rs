//! Esse módulo é responsável pela integração com o "Gnosys", plataforma para
//! autenticar os documentos do SIGA. Aqui, ele é usado somente para autenticar
//! o documento de "Regularmente Matriculado", usado para o cadastro de alunos
//! novos.

use chrono::Local;
use reqwest::ClientBuilder;
use select::document::Document;
use select::predicate::{Attr, Class};
use std::collections::HashMap;
use thiserror::Error;

const GET_URL: &str =
    "https://gnosys.ufrj.br/Documentos/autenticacao/regularmenteMatriculado";
const POST_URL: &str = "https://gnosys.ufrj.br/Documentos/autenticacao.seam";

/// Representa um erro no processo de consulta.
#[derive(Debug, Error)]
pub enum ConsultaErro {
    /// Um erro com a biblioteca de rede. A natureza do erro pode ser vista
    /// acessando ele diretamente. Provavelmente, é um erro de conexão.
    #[error("houve um problema com o reqwest")]
    ErroReqwest(#[from] reqwest::Error),

    /// Um problema ao achar um componente do Gnosys necessário para fazer a
    /// consulta. Isso pode ser uma mudança no Gnosys ou algum erro exibido
    /// em HTTP.
    #[error("não foi possível obter o ViewState")]
    SemViewState,

    /// O retorno do Gnosys não foi inválido nem válido, o que é estranho.
    /// Possivelmente uma mudança por parte do SIGA.
    #[error("o retorno do gnosys está estranho, não é válido nem inválido")]
    CombinacaoInvalida,

    /// A resposta não tem somente três itens (Nome, RG, Curso), o que pode
    /// indicar uma mudança no Gnosys, que agora exibe mais ou menos campos.
    #[error(
        "número estranho de itens na resposta, pode ser uma mudança do gnosys"
    )]
    NumeroEstranhoDeItens,
}

/// Representa o resultado de uma consulta bem-sucedida.
#[derive(Debug)]
pub enum Consulta {
    /// O aluno é do curso de Ciência da Computação e `nome` é seu nome
    /// completo.
    AlunoDoCurso { nome: String },
    /// O aluno é da UFRJ, mas de outro curso. `nome` é seu nome completo e
    /// `curso` é o nome de seu curso.
    AlunoOutroCurso { nome: String, curso: String },
    /// O documento não foi autenticado com sucesso.
    Desconhecido,
}

/// Realiza uma consulta no sistema Gnosys para validar um documento de
/// regularmente matriculado com as informações necessárias:
///
/// - `dre`: 9 dígitos
/// - `data`: Data de emissão no formato `25/12/2025`
/// - `hora`: Hora de emissão no formato `23:59`
/// - `codigo`: código no formato `XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX.XXXX`
///
/// # Errors
///
/// Retorna erro se tiver problemas de conexão ou ao lidar com a plataforma
/// Gnosys.
pub async fn consulta(
    dre: &str,
    data: &str,
    hora: &str,
    codigo: &str,
) -> Result<Consulta, ConsultaErro> {
    let client = ClientBuilder::new().cookie_store(true).build()?;

    let res_form = client.get(GET_URL).send().await?.text().await?;

    let form_doc = Document::from(res_form.as_str());
    let view_state = form_doc
        .find(Attr("name", "javax.faces.ViewState"))
        .next()
        .and_then(|v| v.attr("value"))
        .ok_or(ConsultaErro::SemViewState)?
        .to_string();

    let mes_hoje = Local::now().format("%m/%Y").to_string();

    let mut form = HashMap::new();
    form.insert("AJAXREQUEST", "_viewRoot");
    form.insert("gnosys-filtro_link_hidden_", "gnosys-filtro-campos");
    form.insert("alunoMatricula", dre);
    form.insert("situacaoMatricula", "A");
    form.insert("dataAutenticacaoInputDate", data);
    form.insert("dataAutenticacaoCurrentDate", &mes_hoje);
    form.insert("hora", hora);
    form.insert("assinatura", codigo);
    form.insert("gnosys-filtro", "gnosys-filtro");
    form.insert("autoScroll", "");
    form.insert("javax.faces.ViewState", &view_state);
    form.insert("btnValidarDocumento", "btnValidarDocumento");
    form.insert("", "");

    let res = client
        .post(POST_URL)
        .form(&form)
        .send()
        .await?
        .text()
        .await?;

    let res_doc = Document::from(res.as_str());

    let valido = res_doc
        .find(Attr("id", "msgDocumentoValido"))
        .next()
        .is_some();

    let invalido = res_doc
        .find(Attr("id", "msgDocumentoInvalido"))
        .next()
        .is_some();

    if valido == invalido {
        return Err(ConsultaErro::CombinacaoInvalida);
    }

    if invalido {
        return Ok(Consulta::Desconhecido);
    }

    let consulta: Vec<_> = res_doc
        .find(Class("gnosys-item-visualizacao"))
        .map(|x| x.text())
        .collect();

    if consulta.len() != 3 {
        return Err(ConsultaErro::NumeroEstranhoDeItens);
    }

    if consulta[2] == "Ciência da Computação" {
        Ok(Consulta::AlunoDoCurso {
            nome: consulta[0].clone(),
        })
    } else {
        Ok(Consulta::AlunoOutroCurso {
            nome: consulta[0].clone(),
            curso: consulta[2].clone(),
        })
    }
}
