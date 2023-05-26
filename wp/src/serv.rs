use axum::extract::{Path, RawQuery};
use axum::response::IntoResponse;
use axum::{
    body::Body as AxumBody,
    extract::Extension,
    http::{header::HeaderMap, Request},
    routing::{get, post},
    Router,
};
use clap::Parser;
use leptos::*;
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use wp::backend::chatglm::GLM;
use wp::backend::context::ChatMgr;
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
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .json()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(args.log.parse::<Level>().unwrap_or(Level::INFO))
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // get pwd
    let pwd = std::env::current_dir().unwrap();
    info!(conf_path = &args.config, cwd = ?pwd, "Starting up",);
    info!("Version: {}", env!("COMMIT_ID"));
    let contents =
        fs::read_to_string(&args.config).expect("Should have been able to read the file");
    let serv_conf: backend::Config = toml::from_str(contents.as_str()).unwrap();
    let glm = GLM::new(&serv_conf.glm_api);
    let chat_mgr = Arc::new(Mutex::new(ChatMgr::default()));

    let mp = MP::new(
        &serv_conf.corp_id,
        &serv_conf.corp_secret,
        serv_conf.agent_id.clone(),
        &serv_conf.encoded_aes_key,
        &serv_conf.token,
    );
    let amp = Arc::new(mp);
    let mp_l = amp.clone();

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
        .route(
            "/wccb",
            get(backend::api::validate_url).post(backend::api::on_message),
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
            move |cx| provide_context(cx, mp_l.clone()),
            |cx| view! { cx, <App/> },
        )
        .fallback(file_and_error_handler)
        .layer(Extension(Arc::new(leptos_options)))
        .layer(Extension(chat_mgr))
        .layer(Extension(Arc::new(serv_conf)))
        .layer(Extension(amp))
        .layer(Extension(Arc::new(glm)))
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
    raw_query: RawQuery,
    Extension(mp): Extension<Arc<MP>>,
    Extension(glm): Extension<Arc<GLM>>,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    handle_server_fns_with_context(
        path,
        headers,
        raw_query,
        move |cx| {
            provide_context(cx, mp.clone());
            provide_context(cx, glm.clone());
        },
        request,
    )
    .await
}
