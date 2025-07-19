use crate::cadastro_aluno::DadosParaCadastro;
use axum::Router;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::routing::post;
use serde::Serialize;

#[derive(Serialize)]
struct ResponseBody {
    message: String,
}

async fn cadastrar(
    Json(dados): Json<DadosParaCadastro>,
) -> (StatusCode, Json<ResponseBody>) {
    println!("Recebido {dados:#?}");
    println!();
    println!();

    (
        StatusCode::OK,
        Json(ResponseBody {
            message: "sucesso :)".to_string(),
        }),
    )
}

pub async fn main(address: String) {
    let app = Router::new().route("/api/cadastrar", post(cadastrar));

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
