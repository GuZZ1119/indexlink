use std::future::Future;

pub(crate) async fn signal() {
    signal_with_notifier(None).await;
}

/// Install the supported OS signal handlers, then wait until either one resolves.
///
/// The optional notifier exists only to let the focused Unix test send SIGTERM after both
/// handlers have been installed, rather than racing the process startup.
async fn signal_with_notifier(ready: Option<tokio::sync::oneshot::Sender<()>>) {
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

    if let Some(ready) = ready {
        let _ = ready.send(());
    }

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
    use std::{future::pending, process::Command, time::Duration};

    use super::{signal_with_notifier, wait_for_signal};

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

    /// Verify the real Unix SIGTERM registration path returns after handler installation.
    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn installed_sigterm_handler_releases_shutdown_signal() {
        let (ready, installed) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(signal_with_notifier(Some(ready)));
        installed
            .await
            .expect("shutdown test must install signal handlers before sending SIGTERM");

        let status = Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .expect("system kill command should be available on Unix");
        assert!(status.success());
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("SIGTERM handler should release shutdown task")
            .expect("shutdown task should not panic");
    }
}
