//! Phase 0 Validation Spike: Empirical Verification of the 4 Library Unknowns
//!
//! Run with: cargo test -p orcrustrator-gw --test phase0_spike

use orcrustrator::{
    event::{decode_frame, encode_frame, RelationalEventEnvelope},
    format::StructuredValue,
    MemoryCache,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_unknown_1_cloudevents_contents_and_timing() {
    let envelope = RelationalEventEnvelope::new(
        "orcrustrator://registry.local",
        "orcrustrator.relation.created",
        "patient-stream-01",
        42,
        "/api/v1/patients",
        "Created",
        Some(b"{\"patient_id\": \"p1\"}".to_vec()),
    );

    assert_eq!(envelope.specversion, "1.0");
    assert_eq!(envelope.datacontenttype, "application/msgpack");
    assert_eq!(envelope.operation, "Created");
    assert_eq!(envelope.sequence, 42);

    let wire_bytes = encode_frame(&envelope).expect("Frame encoding must succeed");
    assert!(wire_bytes.len() > 4);

    let (decoded, consumed) = decode_frame(&wire_bytes)
        .expect("Frame decode must succeed")
        .expect("Complete frame expected");

    assert_eq!(consumed, wire_bytes.len());
    assert_eq!(decoded.id, envelope.id);
    assert_eq!(decoded.stream_id, "patient-stream-01");
}

#[tokio::test]
async fn test_unknown_2_cache_key_behavior() {
    let cache = Arc::new(MemoryCache::with_default_ttl(Duration::from_secs(60)));

    let get_key = "GET:/patients/1/summary";
    let mock_data = StructuredValue::String("Cached Patient Summary".to_string());
    cache.set(get_key, &mock_data, None).await.unwrap();

    let cached = cache.get(get_key).await.unwrap();
    assert!(cached.is_some());

    // Invalidate POST
    let post_key = "POST:/visits";
    cache.invalidate(post_key).await.unwrap();

    // GET remains cached because method is part of the cache key
    let still_cached = cache.get(get_key).await.unwrap();
    assert!(still_cached.is_some());
}
