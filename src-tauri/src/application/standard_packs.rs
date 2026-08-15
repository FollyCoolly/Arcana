use crate::domain::{
    Pack, PackManifest, RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, ValueType,
    SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const BASIC_PACK_ID: &str = "basic";

/// The ordinary Pack installed into every newly initialized user repository.
/// It has no hidden database privileges; consumers merely recognize its two
/// stable identity RecordDefinition ids.
pub fn basic_pack() -> Pack {
    Pack {
        manifest: PackManifest {
            schema_version: SCHEMA_VERSION,
            id: BASIC_PACK_ID.to_string(),
            name: "基础".to_string(),
            description: Some("Arcana 的基础身份信息".to_string()),
            author: Some("Arcana".to_string()),
            parent_pack_id: None,
            tags: vec![],
        },
        record_definitions: Some(RecordDefinitionFile {
            definitions: vec![
                RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "identity.birth_date".to_string(),
                    name: "生日".to_string(),
                    description: None,
                    value_type: ValueType::Date,
                    unit: None,
                }),
                RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "identity.nickname".to_string(),
                    name: "昵称".to_string(),
                    description: None,
                    value_type: ValueType::String,
                    unit: None,
                }),
            ],
        }),
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
