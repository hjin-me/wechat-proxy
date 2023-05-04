use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    body::Body as AxumBody,
    extract::Extension,
    http::{header::HeaderMap, Request},
    routing::{any, get, post},
    Router,
};
use clap::Parser;
use leptos::*;
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use std::fs;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use wp::backend::mp::MP;
use wp::components::home::*;
use wp::fallback::file_and_error_handler;
use wp::{api, backend};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short, long, default_value = "./wp/config.toml")]
    config: String,
    #[arg(short, long, default_value = "info")]
    log: String,
}

pub async fn serv() {
    let args = Args::parse();
    // a builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(args.log.parse::<Level>().unwrap_or(Level::INFO))
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // get pwd
    let pwd = std::env::current_dir().unwrap();
    info!("Starting up {}, {:?}", &args.config, pwd);
    let contents =
        fs::read_to_string(&args.config).expect("Should have been able to read the file");
    let serv_conf: backend::Config = toml::from_str(contents.as_str()).unwrap();

    let mp = MP::new(
        &serv_conf.corp_id,
        &serv_conf.corp_secret,
        &serv_conf.agent_id,
    );

    api::register_server_functions();

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

    // build our application with a route
    let app = Router::new()
        .layer(CompressionLayer::new())
        .route("/liveness", get(|| async { "I'm alive!" }))
        .route("/readiness", get(|| async { "I'm ready!" }))
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .route("/cgi-bin/message/send", post(backend::api::message_send))
        .route("/cgi-bin/media/upload", post(backend::api::media_upload))
        .route(
            "/cgi-bin/message/recall",
            post(backend::api::message_recall),
        )
        .route(
            "/cgi-bin/gettoken",
            get(|| async {
                r#"{
   "errcode": 0,
   "errmsg": "ok",
   "access_token": "thisisfaketoken",
   "expires_in": 7200
}"#
            }),
        )
        .leptos_routes_with_context(
            leptos_options.clone(),
            routes,
            move |cx| {},
            |cx| view! { cx, <App/> },
        )
        .fallback(file_and_error_handler)
        .layer(Extension(Arc::new(leptos_options)))
        .layer(Extension(Arc::new(serv_conf)))
        .layer(Extension(Arc::new(mp)))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new()),
        );

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("listening on http://{}", &addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn server_fn_handler(
    path: Path<String>,
    headers: HeaderMap,
    // raw_query: RawQuery,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    handle_server_fns_with_context(path, headers, move |cx| {}, request).await
}
