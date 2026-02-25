use crate::types::*;
use crate::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Convert a flattened ResInfo structure to a hierarchical ResourceGroup.
pub fn convert_res_info_to_resource_group(res_info: &ResInfo) -> Result<ResourceGroup> {
    let mut resource_group = ResourceGroup {
        version: Some(1),
        content_version: Some(1),
        slot_count: 0,
        groups: Vec::new(),
    };

    for (composite_name, group) in &res_info.groups {
        if group.is_composite {
            // Add the composite definition
            let mut composite_k = ShellSubgroupData {
                id: composite_name.clone(),
                r#type: "composite".to_string(),
                res: None,
                parent: None,
                subgroups: Some(Vec::new()),
                resources: None,
            };

            for (subgroup_name, subgroup_data) in &group.subgroup {
                let res_type = subgroup_data.r#type.clone();
                if let Some(subgroups) = composite_k.subgroups.as_mut() {
                    subgroups.push(SubgroupWrapper {
                        id: subgroup_name.clone(),
                        res: res_type.clone(),
                    });
                }
            }
            resource_group.groups.push(composite_k);

            // Add the children
            for (subgroup_name, subgroup_data) in &group.subgroup {
                if let Some(ref r#type) = subgroup_data.r#type {
                    if r#type != "0" {
                        // Likely an atlas/image type packet
                        resource_group.groups.push(generate_image_info(
                            subgroup_name,
                            Some(composite_name),
                            subgroup_data,
                            &res_info.expand_path,
                        )?);
                    } else {
                        // Fallback simple file type packet
                        resource_group.groups.push(generate_file_info(
                            subgroup_name,
                            Some(composite_name),
                            subgroup_data,
                            &res_info.expand_path,
                        )?);
                    }
                } else {
                    resource_group.groups.push(generate_file_info(
                        subgroup_name,
                        Some(composite_name),
                        subgroup_data,
                        &res_info.expand_path,
                    )?);
                }
            }
        } else {
            // Independent items
            for (subgroup_name, subgroup_data) in &group.subgroup {
                resource_group.groups.push(generate_file_info(
                    subgroup_name,
                    None,
                    subgroup_data,
                    &res_info.expand_path,
                )?);
            }
        }
    }

    rewrite_slot(&mut resource_group);
    Ok(resource_group)
}

fn generate_image_info(
    id: &str,
    parent: Option<&String>,
    subgroup_data: &MSubgroupData,
    _expand_path: &str,
) -> Result<ShellSubgroupData> {
    let mut composite_k = ShellSubgroupData {
        id: id.to_string(),
        parent: parent.cloned(),
        res: subgroup_data.r#type.clone(),
        r#type: "simple".to_string(),
        subgroups: None,
        resources: Some(Vec::new()),
    };

    // packet is expected to be a map of AtlasWrapper
    if let Value::Object(packet_map) = &subgroup_data.packet {
        for (key, value) in packet_map {
            // value is AtlasWrapper JSON
            let atlas: AtlasWrapper = serde_json::from_value(value.clone())?;

            let resource = MSubgroupWrapper {
                slot: 0,
                id: key.clone(),
                path: atlas.path,
                r#type: atlas.r#type,
                atlas: Some(true),
                runtime: Some(true),
                width: Some(atlas.dimension.width),
                height: Some(atlas.dimension.height),
                x: None,
                y: None,
                cols: None,
                rows: None,
                parent: None,
                ax: None,
                ay: None,
                aw: None,
                ah: None,
                force_original_vector_symbol_size: None,
                srcpath: None,
            };

            if let Some(resources) = composite_k.resources.as_mut() {
                resources.push(resource);

                for (sub_key, sub_value) in atlas.data {
                    let sub_resource = MSubgroupWrapper {
                        slot: 0,
                        id: sub_key,
                        path: sub_value.path,
                        r#type: sub_value.r#type,
                        parent: Some(key.clone()),
                        ax: Some(sub_value.r#default.ax),
                        ay: Some(sub_value.r#default.ay),
                        aw: Some(sub_value.r#default.aw),
                        ah: Some(sub_value.r#default.ah),
                        x: if sub_value.r#default.x != 0 {
                            Some(sub_value.r#default.x)
                        } else {
                            None
                        },
                        y: if sub_value.r#default.y != 0 {
                            Some(sub_value.r#default.y)
                        } else {
                            None
                        },
                        cols: sub_value.r#default.cols,
                        rows: sub_value.r#default.rows,
                        atlas: None,
                        runtime: None,
                        width: None,
                        height: None,
                        force_original_vector_symbol_size: None,
                        srcpath: None,
                    };
                    resources.push(sub_resource);
                }
            }
        }
    }

    Ok(composite_k)
}

fn generate_file_info(
    id: &str,
    parent: Option<&String>,
    subgroup_data: &MSubgroupData,
    _expand_path: &str,
) -> Result<ShellSubgroupData> {
    let mut composite_k = ShellSubgroupData {
        id: id.to_string(),
        parent: parent.cloned(),
        r#type: "simple".to_string(),
        res: None,
        subgroups: None,
        resources: Some(Vec::new()),
    };

    // packet is expected to be a CommonWrapper
    if let Value::Object(packet_map) = &subgroup_data.packet {
        if let Some(Value::Object(data_map)) = packet_map.get("data") {
            for (key, value) in data_map {
                let data: CommonDataWrapper = serde_json::from_value(value.clone())?;
                let resource = MSubgroupWrapper {
                    slot: 0,
                    id: key.clone(),
                    path: data.path,
                    r#type: data.r#type,
                    srcpath: data.srcpath,
                    force_original_vector_symbol_size: data.force_original_vector_symbol_size,
                    atlas: None,
                    runtime: None,
                    width: None,
                    height: None,
                    x: None,
                    y: None,
                    cols: None,
                    rows: None,
                    parent: None,
                    ax: None,
                    ay: None,
                    aw: None,
                    ah: None,
                };
                if let Some(resources) = composite_k.resources.as_mut() {
                    resources.push(resource);
                }
            }
        }
    }

    Ok(composite_k)
}

/// Convert a hierarchical ResourceGroup back to a flattened ResInfo structure
pub fn convert_resource_group_to_res_info(
    resource_group: &ResourceGroup,
    expand_path_str: &str,
) -> Result<ResInfo> {
    let mut res_info = ResInfo {
        expand_path: expand_path_str.to_string(),
        groups: HashMap::new(),
    };

    for group in &resource_group.groups {
        if let Some(subgroups) = &group.subgroups {
            // It's a composite
            let mut subgroup_dict = HashMap::new();
            for sub in subgroups {
                let found_group = resource_group.groups.iter().find(|g| g.id == sub.id);
                if let Some(found) = found_group {
                    if sub.res.is_some() && sub.res.as_deref() != Some("0") {
                        subgroup_dict.insert(sub.id.clone(), convert_atlas_subgroup_data(found)?);
                    } else {
                        subgroup_dict.insert(sub.id.clone(), convert_common_subgroup_data(found)?);
                    }
                }
            }

            res_info.groups.insert(
                group.id.clone(),
                GroupDictionary {
                    is_composite: true,
                    subgroup: subgroup_dict,
                },
            );
        } else if group.parent.is_none() && group.resources.is_some() {
            // Independent subgroup
            let mut subgroup_dict = HashMap::new();
            subgroup_dict.insert(group.id.clone(), convert_common_subgroup_data(group)?);

            res_info.groups.insert(
                group.id.clone(),
                GroupDictionary {
                    is_composite: false,
                    subgroup: subgroup_dict,
                },
            );
        }
    }

    Ok(res_info)
}

fn convert_atlas_subgroup_data(subgroup: &ShellSubgroupData) -> Result<MSubgroupData> {
    let mut packet = HashMap::new();
    let mut children_by_parent: HashMap<String, Vec<&MSubgroupWrapper>> = HashMap::new();

    if let Some(resources) = &subgroup.resources {
        // First pass: collect children
        for res in resources {
            if let Some(parent_id) = &res.parent {
                children_by_parent
                    .entry(parent_id.clone())
                    .or_default()
                    .push(res);
            }
        }

        // Second pass: build atlas wrappers
        for res in resources {
            if res.atlas == Some(true) {
                let mut atlas = AtlasWrapper {
                    r#type: res.r#type.clone(),
                    path: res.path.clone(),
                    dimension: Dimension {
                        width: res.width.unwrap_or(0),
                        height: res.height.unwrap_or(0),
                    },
                    data: HashMap::new(),
                };

                if let Some(children) = children_by_parent.get(&res.id) {
                    for child in children {
                        atlas.data.insert(
                            child.id.clone(),
                            SpriteData {
                                r#type: child.r#type.clone(),
                                path: child.path.clone(),
                                r#default: DefaultProperty {
                                    ax: child.ax.unwrap_or(0),
                                    ay: child.ay.unwrap_or(0),
                                    aw: child.aw.unwrap_or(0),
                                    ah: child.ah.unwrap_or(0),
                                    x: child.x.unwrap_or(0),
                                    y: child.y.unwrap_or(0),
                                    cols: child.cols,
                                    rows: child.rows,
                                },
                            },
                        );
                    }
                }
                packet.insert(res.id.clone(), atlas);
            }
        }
    }

    Ok(MSubgroupData {
        r#type: subgroup.res.clone(),
        packet: serde_json::to_value(packet)?,
    })
}

fn convert_common_subgroup_data(subgroup: &ShellSubgroupData) -> Result<MSubgroupData> {
    let mut data_map = HashMap::new();

    if let Some(resources) = &subgroup.resources {
        for res in resources {
            data_map.insert(
                res.id.clone(),
                CommonDataWrapper {
                    r#type: res.r#type.clone(),
                    path: res.path.clone(),
                    force_original_vector_symbol_size: res.force_original_vector_symbol_size,
                    srcpath: res.srcpath.clone(),
                },
            );
        }
    }

    let wrapper = CommonWrapper {
        r#type: "File".to_string(),
        data: data_map,
    };

    Ok(MSubgroupData {
        r#type: None,
        packet: serde_json::to_value(wrapper)?,
    })
}

/// Helper fn to reassign incremental slots across unique resource objects globally within the group
fn rewrite_slot(resource_group: &mut ResourceGroup) {
    let mut id_map: HashMap<String, u32> = HashMap::new();
    let mut count = 0;

    for group in &mut resource_group.groups {
        if let Some(resources) = &mut group.resources {
            for res in resources {
                if let Some(&slot) = id_map.get(&res.id) {
                    res.slot = slot;
                } else {
                    res.slot = count;
                    id_map.insert(res.id.clone(), count);
                    count += 1;
                }
            }
        }
    }

    resource_group.slot_count = count;
}
