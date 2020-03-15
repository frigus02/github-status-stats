#[cfg(unix)]
use tokio::stream::StreamExt;

#[cfg(unix)]
pub async fn ctrl_c() {
    let sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
    let sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    sigint.merge(sigterm).next().await;
}

#[cfg(windows)]
pub async fn ctrl_c() {
    tokio::signal::ctrl_c().await.ok();
}
