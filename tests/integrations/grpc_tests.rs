use crate::common;
use helium_proto::Region;
use std::time::Duration;

#[tokio::test]
async fn inc_returns_incrementing_count() {
    let settings = common::test_settings();
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res1 = common::inc(&mut client, "key1", vec![], 0).await;
    assert_eq!(res1.count, 1);
    assert!(!res1.denied);

    let res2 = common::inc(&mut client, "key1", vec![], 0).await;
    assert_eq!(res2.count, 2);

    let res3 = common::inc(&mut client, "key1", vec![], 0).await;
    assert_eq!(res3.count, 3);
}

#[tokio::test]
async fn different_keys_have_independent_counters() {
    let settings = common::test_settings();
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res_a = common::inc(&mut client, "keyA", vec![], 0).await;
    assert_eq!(res_a.count, 1);

    let res_b = common::inc(&mut client, "keyB", vec![], 0).await;
    assert_eq!(res_b.count, 1);

    let res_a2 = common::inc(&mut client, "keyA", vec![], 0).await;
    assert_eq!(res_a2.count, 2);

    let res_b2 = common::inc(&mut client, "keyB", vec![], 0).await;
    assert_eq!(res_b2.count, 2);
}

#[tokio::test]
async fn server_shuts_down_gracefully() {
    let settings = common::test_settings();
    let addr = common::available_port().await;
    let shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res = common::inc(&mut client, "key1", vec![], 0).await;
    assert_eq!(res.count, 1);

    shutdown.trigger();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let result = client
        .inc(helium_proto::services::multi_buy::MultiBuyIncReqV1 {
            key: "key1".to_string(),
            hotspot_key: vec![],
            region: 0,
        })
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn concurrent_requests_on_same_key() {
    let settings = common::test_settings();
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;

    let n = 100u32;
    let mut handles = Vec::with_capacity(n as usize);

    for _ in 0..n {
        handles.push(tokio::spawn(async move {
            let mut client = common::connect_client(addr).await;
            common::inc(&mut client, "concurrent-key", vec![], 0).await
        }));
    }

    let mut counts: Vec<u32> = Vec::with_capacity(n as usize);
    for handle in handles {
        counts.push(handle.await.unwrap().count);
    }

    counts.sort();
    // Every count from 1..=n should appear exactly once (no lost updates)
    assert_eq!(counts, (1..=n).collect::<Vec<_>>());
}

#[tokio::test]
async fn cache_entries_expire_after_cleanup() {
    let cleanup_timeout = Duration::from_millis(200);
    let settings = common::test_settings_with_cleanup(cleanup_timeout);
    let addr = common::available_port().await;
    let _shutdown = common::start_server_with_cleanup(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // Insert an entry
    let res = common::inc(&mut client, "expire-key", vec![], 0).await;
    assert_eq!(res.count, 1);

    // Wait for cleanup to run (cleanup_timeout + margin)
    tokio::time::sleep(cleanup_timeout + Duration::from_millis(300)).await;

    // After expiry, a new inc should start from 1 again
    let res2 = common::inc(&mut client, "expire-key", vec![], 0).await;
    assert_eq!(res2.count, 1, "counter should reset after cache expiry");
}

#[tokio::test]
async fn denied_region_returns_denied_response() {
    let settings = common::test_settings_with_deny_lists(vec![], vec!["EU868".to_string()]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // Request with denied region should be denied but still increment
    let res = common::inc(&mut client, "key1", vec![], Region::Eu868 as i32).await;
    assert!(res.denied);
    assert_eq!(res.count, 1);

    let res2 = common::inc(&mut client, "key1", vec![], Region::Eu868 as i32).await;
    assert!(res2.denied);
    assert_eq!(res2.count, 2);

    // Request with non-denied region should not be denied
    let res3 = common::inc(&mut client, "key2", vec![], Region::As9231 as i32).await;
    assert!(!res3.denied);
    assert_eq!(res3.count, 1);
}

#[tokio::test]
async fn denied_hotspot_returns_denied_response() {
    let hotspot_b58 = "112bUuQaE7j73THS9ABShHGokm46Miip9L361FSyWv7zSYn8hZWf".to_string();
    let hotspot_bytes = hotspot_b58.as_bytes().to_vec();

    let settings = common::test_settings_with_deny_lists(vec![hotspot_b58], vec![]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // Request with denied hotspot should be denied but still increment
    let res = common::inc(&mut client, "key1", hotspot_bytes.clone(), 0).await;
    assert!(res.denied);
    assert_eq!(res.count, 1);

    // Request with different hotspot should not be denied
    let res2 = common::inc(&mut client, "key2", vec![1, 2, 3], 0).await;
    assert!(!res2.denied);
    assert_eq!(res2.count, 1);
}

/// Simulate the exact payload HPR sends after the b58 encoding change.
///
/// HPR builds the request as:
///   key      = hpr_utils:bin_to_hex_string(PacketHash)   → hex string
///   hotspot  = erlang:list_to_binary(libp2p_crypto:bin_to_b58(PubKeyBin)) → b58check string as bytes
///   region   = Region atom (e.g. 'US915') → proto enum value
#[tokio::test]
async fn denied_hotspot_matches_hpr_payload() {
    // A real Helium hotspot address (base58check-encoded public key).
    // This is exactly what HPR now sends as hotspot_key bytes.
    let hotspot_b58 = "13QZwkEXgjE3WzWzy6DvJ1dqKsZM5s3fc4pkFpFb2yME2nRRnJv";

    // Configure the deny list with the same b58 address (as operators would).
    let settings = common::test_settings_with_deny_lists(vec![hotspot_b58.to_string()], vec![]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // Build the request exactly as HPR does:
    //   key = hex-encoded packet hash
    //   hotspot_key = b58 address string as raw bytes
    //   region = proto enum value
    let packet_hash_hex = "a1b2c3d4e5f6".to_string();
    let hotspot_key_bytes = hotspot_b58.as_bytes().to_vec();
    let region = Region::Us915 as i32;

    let res = common::inc(
        &mut client,
        &packet_hash_hex,
        hotspot_key_bytes.clone(),
        region,
    )
    .await;
    assert!(res.denied, "hotspot in deny list should be denied");
    assert_eq!(res.count, 1);

    // Same packet hash from a different hotspot should NOT be denied.
    let other_hotspot = "11z69eJ3czc92k6snrfR1ENqbHP9bovzR4RNiB9qTDs4JDYiY3R";
    let res2 = common::inc(
        &mut client,
        &packet_hash_hex,
        other_hotspot.as_bytes().to_vec(),
        region,
    )
    .await;
    assert!(
        !res2.denied,
        "hotspot not in deny list should not be denied"
    );
    assert_eq!(res2.count, 2);

    // Empty hotspot key (e.g. missing gateway info) should NOT be denied.
    let res3 = common::inc(&mut client, "other-key", vec![], region).await;
    assert!(!res3.denied, "empty hotspot key should not be denied");
}

#[tokio::test]
async fn denied_region_us915() {
    let settings = common::test_settings_with_deny_lists(vec![], vec!["US915".to_string()]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // US915 is proto enum value 0 — must still be denied
    let res = common::inc(&mut client, "key1", vec![], Region::Us915 as i32).await;
    assert!(res.denied, "US915 (region value 0) should be denied");
    assert_eq!(res.count, 1);
}
