//! Example: Adding hotspot and region allow lists to the MultiBuy service.
//!
//! This is the inverse of the deny list approach — only explicitly allowed
//! hotspots or regions are accepted. Everything else is denied.
//!
//! To integrate this into the service you would:
//!
//! 1. Add `allowed_hotspots` and `allowed_regions` fields to `Settings`:
//!
//!    ```rust,ignore
//!    /// Base58-encoded hotspot public keys to allow (empty = allow all)
//!    #[serde(default)]
//!    pub allowed_hotspots: Vec<String>,
//!    /// Region names to allow (empty = allow all, e.g., "US915", "EU868")
//!    #[serde(default)]
//!    pub allowed_regions: Vec<String>,
//!    ```
//!
//! 2. Add the parsed allow sets to `State` and check them in the `inc()` handler
//!    (see the code below).
//!
//! 3. Optionally add a `multi_buy_denied_total` counter metric to track denials.

use helium_proto::services::multi_buy::MultiBuyIncReqV1;
use helium_proto::Region;
use std::collections::HashSet;

/// Parsed allow lists ready for O(1) lookups.
/// When a list is empty, all values are allowed (no restriction).
pub struct AllowLists {
    /// Decoded hotspot public keys to allow. Empty means allow all.
    pub hotspots: HashSet<Vec<u8>>,
    /// Proto region enum values to allow. Empty means allow all.
    pub regions: HashSet<i32>,
}

impl AllowLists {
    /// Parse allow lists from raw config values.
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
                .ok_or_else(|| anyhow::anyhow!("unknown region in allow list: '{}'", name))?;
            regions.insert(region as i32);
        }

        Ok(Self { hotspots, regions })
    }

    pub fn is_denied(&self, req: &MultiBuyIncReqV1) -> bool {
        let hotspot_allowed = self.hotspots.contains(&req.hotspot_key);
        let region_allowed = self.regions.contains(&req.region);

        !hotspot_allowed || !region_allowed
    }
}

fn main() {
    // Example: only allow US915 and EU868 regions
    let hotspots: Vec<String> = vec![];
    let regions = vec!["US915".to_string(), "EU868".to_string()];

    let allow_lists = AllowLists::from_config(&hotspots, &regions).expect("valid allow lists");

    println!(
        "Allow lists loaded: {} hotspot(s), {} region(s)",
        allow_lists.hotspots.len(),
        allow_lists.regions.len()
    );
    println!("(empty hotspot list = all hotspots allowed)");

    // In your State::inc() handler, check:
    //   let denied = allow_lists.is_denied(&request);
    //   if denied {
    //       metrics::counter!("multi_buy_denied_total").increment(1);
    //   }
}
