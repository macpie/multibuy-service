use helium_proto::services::multi_buy::MultiBuyIncReqV1;
use helium_proto::Region;
use std::collections::HashSet;

/// Parsed deny lists ready for O(1) lookups.
pub struct DenyLists {
    /// Decoded hotspot public keys to deny.
    hotspots: HashSet<Vec<u8>>,
    /// Proto region enum values to deny.
    regions: HashSet<i32>,
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
        if self.hotspots.contains(&req.hotspot_key[..]) {
            return true;
        }
        if self.regions.contains(&req.region) {
            return true;
        }
        false
    }
}
