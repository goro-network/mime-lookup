use crate::tables::MimeInfoShared;
use actix_cors::Cors;
use actix_web::middleware::Logger as ActixLogger;
use actix_web::web::{route, Data as ActixData, Path as HttpRequestPath, PayloadConfig};
use actix_web::{
    get as http_get, App as HttpEndpoints, HttpResponse, HttpServer,
    Responder as HttpResponseConverter,
};

const MAX_PAYLOAD_SIZE: usize = 2 * 1024 * 1024;
const SOCKET_BIND: &str = "0.0.0.0:8383";

async fn reject_unmapped_handler() -> impl HttpResponseConverter {
    HttpResponse::Forbidden()
}

#[http_get("/mime")]
async fn get_all_mime_hash(shared_state: ActixData<MimeInfoShared>) -> impl HttpResponseConverter {
    let result = shared_state.get_all_mime_to_hash().await;
    let result_json = serde_json::to_string(&result).unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(result_json)
}

#[http_get("/hash")]
async fn get_all_hash_mime(shared_state: ActixData<MimeInfoShared>) -> impl HttpResponseConverter {
    let result = shared_state.get_all_hash_to_mime().await;
    let result_json = serde_json::to_string(&result).unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(result_json)
}

#[http_get("/mime/{hash_hex}")]
async fn get_mime_by_hash(
    shared_state: ActixData<MimeInfoShared>,
    hash_hex: HttpRequestPath<String>,
) -> impl HttpResponseConverter {
    match shared_state.get_mime_by_hash(&hash_hex).await {
        None => HttpResponse::NotFound().finish(),
        Some(result) => HttpResponse::Ok().body(result),
    }
}

#[http_get("/hash/{mime_identifier}")]
async fn get_hash_of_mime(
    shared_state: ActixData<MimeInfoShared>,
    mime_identifier: HttpRequestPath<String>,
) -> impl HttpResponseConverter {
    match shared_state.get_hash_of_mime(&mime_identifier).await {
        None => HttpResponse::NotFound().finish(),
        Some(result) => HttpResponse::Ok().body(result),
    }
}

pub(crate) async fn run_http_server(shared_state: ActixData<MimeInfoShared>) {
    let payload_config = PayloadConfig::new(MAX_PAYLOAD_SIZE);

    HttpServer::new(move || {
        HttpEndpoints::new()
            .app_data(shared_state.clone())
            .app_data(payload_config.clone())
            .wrap(ActixLogger::default())
            .wrap(Cors::permissive())
            .service(get_all_mime_hash)
            .service(get_all_hash_mime)
            .service(get_mime_by_hash)
            .service(get_hash_of_mime)
            .default_service(route().to(reject_unmapped_handler))
    })
    .disable_signals()
    .bind(SOCKET_BIND)
    .expect("Bad SOCKET_BIND!")
    .run()
    .await
    .expect("Failed to run HTTP Server!");
}
