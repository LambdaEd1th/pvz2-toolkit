use crate::types::*;
use crate::Result;
use std::collections::HashMap;

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
pub(crate) fn rewrite_slot(resource_group: &mut ResourceGroup) {
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
