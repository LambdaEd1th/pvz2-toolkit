use crate::error::{ResourceError, Result};
use crate::types::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn split_rsb_desc(file_path: &Path, out_dir: &Path) -> Result<()> {
    let data = fs::read(file_path)?;
    let description: ResourcesDescription = serde_json::from_slice(&data)?;

    fs::create_dir_all(out_dir)?;
    let subgroups_dir = out_dir.join("subgroups");
    fs::create_dir_all(&subgroups_dir)?;

    let mut definition = ManifestDefinition {
        groups: BTreeMap::new(),
    };

    for (group_id, group_data) in description.groups {
        let mut def_group = SubgroupDefinition {
            composite: group_data.composite,
            subgroups: Vec::new(),
        };

        for (subgroup_id, subgroup_data) in group_data.subgroups {
            def_group.subgroups.push(subgroup_id.clone());

            let subgroup_file = subgroups_dir.join(format!("{}.json", subgroup_id));
            let json_str = serde_json::to_string_pretty(&subgroup_data)?;
            fs::write(subgroup_file, json_str)?;
        }

        definition.groups.insert(group_id, def_group);
    }

    let definition_file = out_dir.join("definition.json");
    let def_json = serde_json::to_string_pretty(&definition)?;
    fs::write(definition_file, def_json)?;

    Ok(())
}

pub fn merge_rsb_desc(dir_path: &Path, out_file: &Path) -> Result<()> {
    let definition_file = dir_path.join("definition.json");
    if !definition_file.exists() {
        return Err(ResourceError::MissingPath(
            definition_file.to_string_lossy().to_string(),
        ));
    }

    let def_data = fs::read(definition_file)?;
    let definition: ManifestDefinition = serde_json::from_slice(&def_data)?;

    let subgroups_dir = dir_path.join("subgroups");
    let mut description = ResourcesDescription {
        groups: std::collections::HashMap::new(), // Note: ResourcesDescription uses HashMap from rsb crate
    };

    for (group_id, group_def) in definition.groups {
        let mut group_data = DescriptionGroup {
            composite: group_def.composite,
            subgroups: std::collections::HashMap::new(),
        };

        for subgroup_id in group_def.subgroups {
            let subgroup_file = subgroups_dir.join(format!("{}.json", subgroup_id));
            if !subgroup_file.exists() {
                return Err(ResourceError::MissingPath(
                    subgroup_file.to_string_lossy().to_string(),
                ));
            }

            let sub_data = fs::read(subgroup_file)?;
            let subgroup_data: DescriptionSubGroup = serde_json::from_slice(&sub_data)?;

            group_data.subgroups.insert(subgroup_id, subgroup_data);
        }

        description.groups.insert(group_id, group_data);
    }

    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)?;
    }

    let out_json = serde_json::to_string_pretty(&description)?;
    fs::write(out_file, out_json)?;

    Ok(())
}

pub fn split_rsg_res(file_path: &Path, out_dir: &Path) -> Result<()> {
    let data = fs::read(file_path)?;
    let mut manifest: PopCapResourceManifest = serde_json::from_slice(&data)?;

    fs::create_dir_all(out_dir)?;
    let subgroup_dir = out_dir.join("subgroup");
    fs::create_dir_all(&subgroup_dir)?;

    let mut content = ContentJson {
        groups: BTreeMap::new(),
    };

    for group in &mut manifest.groups {
        match group {
            PopCapResourceGroup::Composite(comp) => {
                let mut content_group = ContentGroupDef {
                    is_composite: true,
                    subgroups: BTreeMap::new(),
                };
                for sub in &comp.subgroups {
                    content_group.subgroups.insert(
                        sub.id.clone(),
                        ContentSubgroupDef {
                            res_type: sub.res.clone(),
                        },
                    );
                }
                content.groups.insert(comp.id.clone(), content_group);
            }
            PopCapResourceGroup::Resources(res) => {
                for r in &mut res.resources {
                    if let Some(obj) = r.as_object_mut() {
                        obj.remove("slot");
                    }
                }

                if res.parent.is_some() {
                    let subgroup_file = subgroup_dir.join(format!("{}.json", res.id));
                    let json_str = serde_json::to_string_pretty(res)?;
                    fs::write(subgroup_file, json_str)?;
                }

                if res.parent.is_none() {
                    let mut content_group = ContentGroupDef {
                        is_composite: false,
                        subgroups: BTreeMap::new(),
                    };
                    content_group
                        .subgroups
                        .insert(res.id.clone(), ContentSubgroupDef { res_type: None });
                    content.groups.insert(res.id.clone(), content_group);

                    let subgroup_file = subgroup_dir.join(format!("{}.json", res.id));
                    let json_str = serde_json::to_string_pretty(res)?;
                    fs::write(subgroup_file, json_str)?;
                }
            }
            PopCapResourceGroup::Other(_) => {
                // Ignore extra items not matching our group definitions
            }
        }
    }

    let content_file = out_dir.join("content.json");
    let content_json_str = serde_json::to_string_pretty(&content)?;
    fs::write(content_file, content_json_str)?;

    Ok(())
}

pub fn merge_rsg_res(dir_path: &Path, out_file: &Path) -> Result<()> {
    let content_file = dir_path.join("content.json");
    if !content_file.exists() {
        return Err(ResourceError::MissingPath(
            content_file.to_string_lossy().to_string(),
        ));
    }

    let content_data = fs::read(content_file)?;
    let content: ContentJson = serde_json::from_slice(&content_data)?;

    let subgroup_dir = dir_path.join("subgroup");

    let mut manifest = PopCapResourceManifest {
        version: Some(1),
        content_version: Some(1),
        slot_count: Some(0),
        groups: Vec::new(),
    };

    for (parent_id, group_def) in content.groups {
        if group_def.is_composite {
            let mut composite = ResourceComposite {
                id: parent_id.clone(),
                res_type: "composite".to_string(),
                subgroups: Vec::new(),
                extra: std::collections::BTreeMap::new(),
            };

            for (sub_id, sub_def) in &group_def.subgroups {
                composite.subgroups.push(ResourceSubgroupRef {
                    id: sub_id.clone(),
                    res: sub_def.res_type.clone(),
                });
            }
            manifest
                .groups
                .push(PopCapResourceGroup::Composite(composite));
        }

        for sub_id in group_def.subgroups.keys() {
            let subgroup_file = subgroup_dir.join(format!("{}.json", sub_id));
            if !subgroup_file.exists() {
                return Err(ResourceError::MissingPath(
                    subgroup_file.to_string_lossy().to_string(),
                ));
            }

            let sub_data = fs::read(subgroup_file)?;
            let mut resource_block: ResourceBlock = serde_json::from_slice(&sub_data)?;

            for r in &mut resource_block.resources {
                if let Some(obj) = r.as_object_mut() {
                    obj.insert("slot".to_string(), serde_json::json!(0));
                }
            }

            manifest
                .groups
                .push(PopCapResourceGroup::Resources(resource_block));
        }
    }

    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)?;
    }

    let out_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(out_file, out_json)?;

    Ok(())
}
