use crate::common;
use helium_proto::Region;
use std::time::Duration;

#[tokio::test]
async fn inc_returns_incrementing_count() {
    let settings = common::test_settings(vec![], vec![]);
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
    let settings = common::test_settings(vec![], vec![]);
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
async fn denied_hotspot_returns_denied_and_still_increments() {
    let denied_key = bs58::encode(b"denied-hotspot-1").into_string();
    let settings = common::test_settings(vec![denied_key], vec![]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res = common::inc(&mut client, "key1", b"denied-hotspot-1".to_vec(), 0).await;
    assert!(res.denied);
    assert_eq!(res.count, 1, "counter should still increment when denied");

    let res2 = common::inc(&mut client, "key1", b"denied-hotspot-1".to_vec(), 0).await;
    assert!(res2.denied);
    assert_eq!(res2.count, 2);
}

#[tokio::test]
async fn allowed_hotspot_is_not_denied() {
    let denied_key = bs58::encode(b"denied-hotspot-1").into_string();
    let settings = common::test_settings(vec![denied_key], vec![]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res = common::inc(&mut client, "key1", b"allowed-hotspot".to_vec(), 0).await;
    assert!(!res.denied);
    assert_eq!(res.count, 1);
}

#[tokio::test]
async fn denied_region_returns_denied() {
    let settings = common::test_settings(vec![], vec!["EU868".to_string()]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let eu868 = Region::Eu868 as i32;

    let res = common::inc(&mut client, "key1", vec![], eu868).await;
    assert!(res.denied);
    assert_eq!(res.count, 1);
}

#[tokio::test]
async fn allowed_region_is_not_denied() {
    let settings = common::test_settings(vec![], vec!["EU868".to_string()]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let us915 = Region::Us915 as i32;

    let res = common::inc(&mut client, "key1", vec![], us915).await;
    assert!(!res.denied);
}

#[tokio::test]
async fn empty_hotspot_key_is_not_denied() {
    let denied_key = bs58::encode(b"denied-hotspot-1").into_string();
    let settings = common::test_settings(vec![denied_key], vec![]);
    let addr = common::available_port().await;
    let _shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    let res = common::inc(&mut client, "key1", vec![], 0).await;
    assert!(!res.denied);
}

#[tokio::test]
async fn server_shuts_down_gracefully() {
    let settings = common::test_settings(vec![], vec![]);
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
    let settings = common::test_settings(vec![], vec![]);
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
    let settings = common::test_settings_with_cleanup(vec![], vec![], cleanup_timeout);
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
