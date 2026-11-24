//! OpenRouter Images topic index.

use super::*;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ImageApi;
use opi_ai::api::openrouter_images::{
    OpenRouterImagesApi, OpenRouterImagesOptions, default_openrouter_image_record,
};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::images::{
    ImageCostRates, ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesInput,
    ImagesOutput, ImagesStopReason,
};
use opi_ai::models::Models;
use opi_ai::types::ApiId;
use opi_telemetry::memory::Recorder;

#[path = "images/callbacks.rs"]
mod callbacks;
#[path = "images/fixtures.rs"]
mod fixtures;
#[path = "images/lifecycle.rs"]
mod lifecycle;
#[path = "images/wire.rs"]
mod wire;

pub(crate) use fixtures::{
    image_models, image_record, images_call, spawn_hanging_body_images_server,
    spawn_retry_images_server, text_part_json,
};
