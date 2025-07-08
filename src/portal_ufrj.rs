use chrono::Local;
use reqwest::ClientBuilder;
use select::document::Document;
use select::predicate::{Attr, Class};
use std::collections::HashMap;
use thiserror::Error;

const GET_URL: &str =
    "https://gnosys.ufrj.br/Documentos/autenticacao/regularmenteMatriculado";
const POST_URL: &str = "https://gnosys.ufrj.br/Documentos/autenticacao.seam";

#[derive(Debug, Error)]
pub enum ConsultaErro {
    #[error("houve um problema com o reqwest")]
    ErroReqwest(#[from] reqwest::Error),

    #[error("não foi possível obter o ViewState")]
    SemViewState,

    #[error("o retorno do gnosys está estranho, não é válido nem inválido")]
    CombinacaoInvalida,

    #[error(
        "número estranho de itens na resposta, pode ser uma mudança do gnosys"
    )]
    NumeroEstranhoDeItens,
}

#[derive(Debug)]
pub enum Consulta {
    AlunoDoCurso { nome: String },
    AlunoOutroCurso { nome: String, curso: String },
    Desconhecido,
}

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
