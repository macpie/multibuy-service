//! Example: Adding hotspot and region deny lists to the MultiBuy service.
//!
//! This example shows how to extend the core `State` with deny-list support so
//! that specific hotspots (by base58 public key) or entire regions can be
//! rejected at the gRPC layer.
//!
//! To integrate this into the service you would:
//!
//! 1. Add `denied_hotspots` and `denied_regions` fields to `Settings`:
//!
//!    ```rust,ignore
//!    /// Base58-encoded hotspot public keys to deny
//!    #[serde(default)]
//!    pub denied_hotspots: Vec<String>,
//!    /// Region names to deny (e.g., "US915", "EU868")
//!    #[serde(default)]
//!    pub denied_regions: Vec<String>,
//!    ```
//!
//! 2. Add the parsed deny sets to `State` and check them in the `inc()` handler
//!    (see the code below).
//!
//! 3. Optionally add a `multi_buy_denied_total` counter metric to track denials.

use helium_proto::services::multi_buy::MultiBuyIncReqV1;
use helium_proto::Region;
use std::collections::HashSet;

/// Parsed deny lists ready for O(1) lookups.
pub struct DenyLists {
    /// Decoded hotspot public keys to deny.
    pub hotspots: HashSet<Vec<u8>>,
    /// Proto region enum values to deny.
    pub regions: HashSet<i32>,
}

impl DenyLists {
    /// Parse deny lists from raw config values.
    ///
    /// `hotspot_keys_b58` are base58-encoded public keys.
    /// `region_names` are proto enum names like "US915" or "EU868".
    pub fn from_config(
        hotspot_keys_b58: &[String],
        region_names: &[String],
    ) -> anyhow::Result<Self> {
        let mut hotspots = HashSet::new();
        for key_b58 in hotspot_keys_b58 {
            let decoded = bs58::decode(key_b58)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("invalid base58 hotspot key '{}': {}", key_b58, e))?;
            hotspots.insert(decoded);
        }

        let mut regions = HashSet::new();
        for name in region_names {
            let region = Region::from_str_name(name)
                .ok_or_else(|| anyhow::anyhow!("unknown region in deny list: '{}'", name))?;
            regions.insert(region as i32);
        }

        Ok(Self { hotspots, regions })
    }

    /// Returns `true` if the request should be denied based on hotspot key or region.
    pub fn is_denied(&self, req: &MultiBuyIncReqV1) -> bool {
        if !req.hotspot_key.is_empty() && self.hotspots.contains(&req.hotspot_key[..]) {
            return true;
        }
        if req.region != 0 && self.regions.contains(&req.region) {
            return true;
        }
        false
    }
}

fn main() {
    // Example: parse deny lists from config-like values
    let hotspots = vec!["112bUuQaE7j73THS9ABShHGokm46Miip9L361FSyWv7zSYn8hZWf".to_string()];
    let regions = vec!["EU868".to_string()];

    let deny_lists = DenyLists::from_config(&hotspots, &regions).expect("valid deny lists");

    println!(
        "Deny lists loaded: {} hotspot(s), {} region(s)",
        deny_lists.hotspots.len(),
        deny_lists.regions.len()
    );

    // In your State::inc() handler, check:
    //   let denied = deny_lists.is_denied(&request);
    //   if denied {
    //       metrics::counter!("multi_buy_denied_total").increment(1);
    //   }
}
