use std::future::Future;

pub(crate) async fn signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    wait_for_signal(ctrl_c, terminate).await;

    tracing::info!("shutdown signal received");
}

/// Wait for either supported process shutdown signal.
async fn wait_for_signal<C, T>(ctrl_c: C, terminate: T)
where
    C: Future<Output = ()>,
    T: Future<Output = ()>,
{
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::wait_for_signal;

    /// Verify Ctrl+C completion releases the shutdown waiter without a real OS signal.
    #[tokio::test]
    async fn waits_for_ctrl_c_completion() {
        wait_for_signal(async {}, pending()).await;
    }

    /// Verify SIGTERM completion releases the shutdown waiter without a real OS signal.
    #[tokio::test]
    async fn waits_for_terminate_completion() {
        wait_for_signal(pending(), async {}).await;
    }
}
