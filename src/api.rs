use crate::cadastro_aluno::DadosParaCadastro;
use crate::configuracao::Configuracao;
use axum::Router;
use axum::extract::{Json, State, rejection::JsonRejection};
use axum::http::StatusCode;
use axum::routing::post;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct ResponseBody {
    message: String,
    sabar_mais: Option<String>,
}

async fn cadastrar(
    State(cfg): State<Arc<Configuracao>>,
    dados: Result<Json<DadosParaCadastro>, JsonRejection>,
) -> (StatusCode, Json<ResponseBody>) {
    println!("Recebido {dados:#?}");
    println!();
    println!();

    match dados {
        Ok(Json(dados)) => {
            match dados.cadastrar(
                &cfg.usuario_novo,
                &cfg.ldap_url,
                &cfg.ldap_bind_dn,
                &cfg.ldap_bind_pw,
            ).await {
                Ok(username) => {
                    (
                        StatusCode::OK,
                        Json(ResponseBody {
                            message: format!(
                                "Cadastrado como {username:?} com sucesso",
                            ),
                            sabar_mais: None,
                        }),
                    )
                },
                Err(err) => {
                    (
                        err.status(),
                        Json(ResponseBody {
                            message: format!("Erro: {}", err),
                            sabar_mais: None,
                        }),
                    )
                }
            }
        }
        Err(rej) => {
            (
                rej.status(),
                Json(ResponseBody {
                    message: "Houve um erro interno, por favor tentar novamente mais tarde.".to_string(),
                    sabar_mais: Some(rej.body_text()),
                }),
            )
        }
    }
}

pub async fn main(address: String, cfg: Arc<Configuracao>) {
    let app = Router::new()
        .route("/api/cadastrar", post(cadastrar))
        .with_state(cfg);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
