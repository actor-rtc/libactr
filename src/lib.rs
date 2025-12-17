//! libactr - UniFFI bindings for the Actor-RTC framework
//!
//! This crate provides FFI bindings for the actr framework using Mozilla UniFFI.
//!
//! ## Architecture
//!
//! The actr framework uses complex Rust features (generics, traits, async) that don't map
//! directly to UniFFI. This crate provides a "facade" layer that:
//!
//! 1. Wraps generic types with concrete DynamicWorkload implementation
//! 2. Uses callback interfaces to implement workload logic
//! 3. Exposes simplified APIs for creating and managing actors

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

fn init_logging() {
    INIT.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .init();

        tracing::info!("libactr logging initialized");
    });
}

// Generate UniFFI scaffolding
uniffi::setup_scaffolding!();
