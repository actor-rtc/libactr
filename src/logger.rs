use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the logger for actr.
/// On Android, this outputs to Logcat with tag "actr".
/// On other platforms, this outputs to stdout.
#[uniffi::export]
pub fn init_logger() {
    INIT.call_once(|| {
        #[cfg(target_os = "android")]
        {
            // Use tracing-android to output to Android Logcat
            use tracing_subscriber::layer::SubscriberExt;
            use tracing_subscriber::util::SubscriberInitExt;

            let android_layer =
                tracing_android::layer("actr").expect("Failed to create Android tracing layer");

            let filter =
                tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        "info,actr=debug,libactr=debug,actr_runtime=debug,actr_protocol=debug",
                    )
                });

            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(android_layer)
                .try_init();
        }

        #[cfg(not(target_os = "android"))]
        {
            use tracing_subscriber::EnvFilter;

            let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new(
                    "info,actr=debug,libactr=debug,actr_runtime=debug,actr_protocol=debug",
                )
            });

            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .try_init();
        }
    });
}
