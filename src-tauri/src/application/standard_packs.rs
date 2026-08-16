use crate::domain::{Pack, RecordDefinitionFile};
use std::collections::BTreeMap;

pub const BASIC_PACK_ID: &str = "basic";

/// The ordinary Pack installed into every newly initialized user repository.
/// It has no hidden database privileges; consumers merely recognize its two
/// stable identity RecordDefinition ids.
pub fn basic_pack() -> Pack {
    Pack {
        manifest: serde_json::from_str(include_str!("../../resources/basic-pack/manifest.json"))
            .expect("bundled basic Pack manifest must be valid"),
        record_definitions: Some(
            serde_json::from_str::<RecordDefinitionFile>(include_str!(
                "../../resources/basic-pack/record-definitions.json"
            ))
            .expect("bundled basic RecordDefinitions must be valid"),
        ),
        dimensions: None,
        achievements: None,
        skills: None,
        assets: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Validate;

    #[test]
    fn basic_pack_is_a_valid_ordinary_pack() {
        let pack = basic_pack();
        assert!(pack.validate().is_ok());
        assert_eq!(pack.manifest.id, BASIC_PACK_ID);
        let ids: Vec<_> = pack
            .record_definitions
            .unwrap()
            .definitions
            .into_iter()
            .map(|definition| definition.id().to_string())
            .collect();
        assert_eq!(ids, ["identity.birth_date", "identity.nickname"]);
    }
}
