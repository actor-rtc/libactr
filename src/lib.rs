//! actr-kotlin - UniFFI bindings for the Actor-RTC framework
//!
//! This crate provides Kotlin/Android bindings for the actr framework using Mozilla UniFFI.
//!
//! ## Architecture
//!
//! The actr framework uses complex Rust features (generics, traits, async) that don't map
//! directly to UniFFI. This crate provides a "facade" layer that:
//!
//! 1. Wraps generic types with concrete DynamicWorkload implementation
//! 2. Uses callback interfaces for Kotlin to implement workload logic
//! 3. Exposes simplified APIs for creating and managing actors
//!
//! ## Usage Pattern
//!
//! ```kotlin
//! // 1. Implement WorkloadCallback
//! class EchoService : WorkloadCallback {
//!     override fun actorType() = ActrType("acme", "echo.EchoService")
//!     override fun onRequest(routeKey: String, payload: ByteArray, ctx: CallContext): ByteArray {
//!         // Handle request and return response
//!     }
//!     override fun onStart(ctx: CallContext) { }
//!     override fun onStop() { }
//! }
//!
//! // 2. Create and start actor
//! val config = ActrConfigBuilder()
//!     .signalingUrl("ws://localhost:8081/signaling/ws")
//!     .actorType("acme", "echo.EchoService")
//!     .build()
//!
//! val system = ActrSystemWrapper.create(config)
//! val node = system.attach(EchoService())
//! val actrRef = node.start()
//!
//! // 3. Discover and call other actors
//! val targets = actrRef.discoverSync(ActrType("acme", "other.Service"), 1)
//! val response = ctx.callSync(targets[0], "route.key", requestBytes)
//!
//! // 4. Shutdown
//! actrRef.waitForShutdown()
//! ```

mod error;
mod runtime;
mod types;
mod workload;

pub use error::*;
pub use runtime::*;
pub use types::*;
pub use workload::*;

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize logging for Android
/// This should be called automatically when the library is loaded
#[cfg(target_os = "android")]
fn init_logging() {
    INIT.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let android_layer =
            tracing_android::layer("actr-kotlin").expect("Failed to create Android layer");

        tracing_subscriber::registry()
            .with(android_layer)
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .init();

        tracing::info!("actr-kotlin logging initialized for Android");
    });
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    INIT.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .init();

        tracing::info!("actr-kotlin logging initialized");
    });
}

// Generate UniFFI scaffolding
uniffi::setup_scaffolding!();
