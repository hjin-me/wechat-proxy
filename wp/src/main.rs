#[cfg(feature = "ssr")]
mod serv;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    serv::serv().await;
}
