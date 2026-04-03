use helium_proto::services::multi_buy::MultiBuyIncReqV1;
use helium_proto::Region;
use std::collections::HashSet;

/// Parsed deny lists ready for O(1) lookups.
pub struct DenyLists {
    /// Base58check hotspot addresses to deny.
    hotspots: HashSet<String>,
    /// Proto region enum values to deny.
    regions: HashSet<i32>,
}

impl DenyLists {
    /// Parse deny lists from raw config values.
    ///
    /// `hotspot_keys_b58` are base58check-encoded public keys (matching what HPR
    /// now sends as the `hotspot_key` bytes field).
    /// `region_names` are proto enum names like "US915" or "EU868".
    pub fn from_config(
        hotspot_keys_b58: &[String],
        region_names: &[String],
    ) -> anyhow::Result<Self> {
        let hotspots: HashSet<String> = hotspot_keys_b58
            .iter()
            .filter(|k| !k.is_empty())
            .cloned()
            .collect();

        let mut regions = HashSet::new();
        for name in region_names {
            if name.is_empty() {
                continue;
            }
            let region = Region::from_str_name(name)
                .ok_or_else(|| anyhow::anyhow!("unknown region in deny list: '{}'", name))?;
            regions.insert(region as i32);
        }

        Ok(Self { hotspots, regions })
    }

    /// Returns `true` if the request should be denied based on hotspot key or region.
    pub fn is_denied(&self, req: &MultiBuyIncReqV1) -> bool {
        if let Ok(hotspot_str) = std::str::from_utf8(&req.hotspot_key) {
            if self.hotspots.contains(hotspot_str) {
                return true;
            }
        }
        if self.regions.contains(&req.region) {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_region_is_skipped() {
        let deny = DenyLists::from_config(&[], &["".into()]).unwrap();
        assert!(deny.regions.is_empty());
    }

    #[test]
    fn empty_string_hotspot_is_skipped() {
        let deny = DenyLists::from_config(&["".into()], &[]).unwrap();
        assert!(deny.hotspots.is_empty());
    }
}
