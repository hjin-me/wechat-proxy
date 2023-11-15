use axum::response::IntoResponse;

pub async fn xx_app_caller() -> impl IntoResponse {
    let s = include_str!("xx.html");
    s.to_string()
}
