use crate::common;
use helium_proto::Region;

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

    // Request with denied hotspot key
    let res = common::inc(&mut client, "key1", b"denied-hotspot-1".to_vec(), 0).await;
    assert!(res.denied);
    assert_eq!(res.count, 1, "counter should still increment when denied");

    // Second request with same denied hotspot
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

    // Request with a different (allowed) hotspot key
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

    // Empty hotspot key should never match deny list
    let res = common::inc(&mut client, "key1", vec![], 0).await;
    assert!(!res.denied);
}

#[tokio::test]
async fn server_shuts_down_gracefully() {
    let settings = common::test_settings(vec![], vec![]);
    let addr = common::available_port().await;
    let shutdown = common::start_server(&settings, addr).await;
    let mut client = common::connect_client(addr).await;

    // Confirm server is alive
    let res = common::inc(&mut client, "key1", vec![], 0).await;
    assert_eq!(res.count, 1);

    // Trigger shutdown
    shutdown.trigger();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Client should get a connection error
    let result = client
        .inc(helium_proto::services::multi_buy::MultiBuyIncReqV1 {
            key: "key1".to_string(),
            hotspot_key: vec![],
            region: 0,
        })
        .await;
    assert!(result.is_err());
}
