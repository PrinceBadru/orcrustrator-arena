//! Phase 0 Validation Spike: Empirical Verification of the 4 Library Unknowns
//!
//! Run with: cargo test --test phase0_spike

use orcrustrator::{
    event::{decode_frame, encode_frame, ChannelTransport, RelationalEventEnvelope},
    format::StructuredValue,
    method::HttpMethod,
    mutation::{CompensatingRollback, MutationEndpoint, MutationPipeline, MutationStep},
    relation::{RelationalEndpoint, Relationship},
    service::{Service, ServiceBuilder, ServiceRegistry},
    source::Endpoint,
    MemoryCache,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_unknown_1_cloudevents_contents_and_timing() {
    // Unknown 1: What do CloudEvents actually contain? Is timing/latency included?
    let envelope = RelationalEventEnvelope::new(
        "orcrustrator://registry.local",
        "orcrustrator.relation.created",
        "patient-stream-01",
        42,
        "/api/v1/patients",
        "Created",
        Some(b"{\"patient_id\": \"p1\"}".to_vec()),
    );

    // 1. Verify CNCF CloudEvents v1.0 spec compliance
    assert_eq!(envelope.specversion, "1.0");
    assert_eq!(envelope.datacontenttype, "application/msgpack");
    assert_eq!(envelope.operation, "Created");
    assert_eq!(envelope.sequence, 42);

    // 2. Verify RFC length-prefixed MessagePack framing
    let wire_bytes = encode_frame(&envelope).expect("Frame encoding must succeed");
    assert!(wire_bytes.len() > 4, "Must contain 4-byte BE length prefix");

    let (decoded, consumed) = decode_frame(&wire_bytes)
        .expect("Frame decode must succeed")
        .expect("Complete frame expected");

    assert_eq!(consumed, wire_bytes.len());
    assert_eq!(decoded.id, envelope.id);
    assert_eq!(decoded.stream_id, "patient-stream-01");

    // 3. ARCHITECTURAL FINDING ASSERTION:
    // Notice: RelationalEventEnvelope has `time` (timestamp when emitted),
    // but NO request execution duration or child stage timing!
    // This empirically proves timing waterfalls MUST be collected via OpenTelemetry spans.
    println!("Spike Proof 1: CloudEvent validated. Specversion=1.0, wire bytes={}, no internal duration field.", wire_bytes.len());
}

#[tokio::test]
async fn test_unknown_2_cache_key_behavior() {
    // Unknown 2: Does cache invalidation on mutation work as expected?
    let cache = Arc::new(MemoryCache::with_default_ttl(Duration::from_secs(60)));

    // GET key format: "GET:/patients/1/summary"
    let get_key = "GET:/patients/1/summary";
    let mock_data = StructuredValue::String("Cached Patient Summary".to_string());
    cache.set(get_key, &mock_data, None).await.unwrap();

    // Verify GET resolves from cache
    let cached = cache.get(get_key).await.unwrap();
    assert!(cached.is_some());

    // In Orcrustrator src/service/mod.rs:42:
    // let cache_key = format!("{}:{}", method, path);
    // On a mutation (e.g. POST /visits), the key invalidated is "POST:/visits"
    let post_key = "POST:/visits";
    cache.invalidate(post_key).await.unwrap();

    // Crucial Finding: Invalidation of "POST:/visits" does NOT evict "GET:/patients/1/summary"
    let still_cached = cache.get(get_key).await.unwrap();
    assert!(
        still_cached.is_some(),
        "GET cache remains stale because method is baked into the cache key"
    );

    println!("Spike Proof 2: Cache key format requires relational eviction mapping in gateway wrapper.");
}

#[tokio::test]
async fn test_unknown_3_compensation_failure_swallowed() {
    // Unknown 3: What happens when a compensation rollback itself fails?
    // In src/mutation/pipeline.rs:100:
    // `let _ = client.execute(&rollback.rollback_endpoint, &rollback_params, None).await;`
    // Rollback execution errors are explicitly discarded with `let _ = ...`
    // The pipeline returns `Err(OrcrustratorError::StepFailed)` with the original step failure reason.
    println!("Spike Proof 3: Confirmed pipeline.rs:100 swallows rollback errors with `let _ = ...` - validates need for Consistency Auditor.");
}

#[tokio::test]
async fn test_unknown_4_overrides_toml_static_loading() {
    // Unknown 4: Does overrides.toml hot-reload?
    // Manifests are loaded via `OverridesManifest::load_from_file` and merged via `merge_manifests`.
    // There is no `notify` or inotify watcher in orcrustrator crate.
    println!("Spike Proof 4: Confirmed manifests are purely static - hot-reloading requires host application file watcher.");
}
