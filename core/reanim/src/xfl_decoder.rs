use crate::{Reanim, ReanimTrack, ReanimTransform};
use anyhow::{Context, Result, bail};
use roxmltree::{Document, Node};
use std::fs;
use std::path::Path;

pub fn decode_xfl(input_dir: &Path) -> Result<Reanim> {
    let dom_doc_path = input_dir.join("DOMDocument.xml");
    let xml_data = fs::read_to_string(&dom_doc_path)
        .with_context(|| format!("Failed to read {:?}", dom_doc_path))?;

    let doc = Document::parse(&xml_data)?;
    let root = doc.root_element();

    if root.tag_name().name() != "DOMDocument" {
        bail!("Invalid DOMDocument");
    }

    let mut reanim = Reanim::default();
    reanim.fps = root
        .attribute("frameRate")
        .unwrap_or("30")
        .parse()
        .unwrap_or(30.0);

    let timelines = root
        .children()
        .find(|n| n.has_tag_name("timelines"))
        .context("Missing <timelines>")?;

    let dom_timeline = timelines
        .children()
        .find(|n| n.has_tag_name("DOMTimeline"))
        .context("Missing <DOMTimeline>")?;

    if dom_timeline.attribute("name") != Some("animation") {
        bail!("Invalid DOMTimeline name, expected 'animation'");
    }

    let layers_node = dom_timeline
        .children()
        .find(|n| n.has_tag_name("layers"))
        .context("Missing <layers>")?;

    let mut dom_layers: Vec<Node> = layers_node
        .children()
        .filter(|n| n.has_tag_name("DOMLayer"))
        .collect();
    dom_layers.reverse();

    let mut tracks = Vec::new();

    for dom_layer in dom_layers {
        let layer_name = dom_layer
            .attribute("name")
            .context("DOMLayer missing name")?
            .to_string();

        let frames_node = dom_layer
            .children()
            .find(|n| n.has_tag_name("frames"))
            .context("DOMLayer missing frames")?;

        let dom_frames: Vec<Node> = frames_node
            .children()
            .filter(|n| n.has_tag_name("DOMFrame"))
            .collect();

        let mut transforms = Vec::new();

        let mut default_transform = ReanimTransform {
            x: Some(0.0),
            y: Some(0.0),
            sx: Some(1.0),
            sy: Some(1.0),
            kx: Some(0.0),
            ky: Some(0.0),
            f: Some(0.0),
            a: Some(1.0),
            ..Default::default()
        };

        let mut temp_name = "-1".to_string();
        let mut frame_switch = false;

        // C# Sen checks `frame_duration` logic. In XFL, consecutive identical frames are grouped
        // using `duration` attribute. Sen's reanim stores EVERY frame explicitly.
        // We need to unroll `duration`.
        for dom_frame in dom_frames {
            let duration = dom_frame
                .attribute("duration")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);

            let elements = dom_frame.children().find(|n| n.has_tag_name("elements"));
            let symbol_instance =
                elements.and_then(|e| e.children().find(|n| n.has_tag_name("DOMSymbolInstance")));

            for frame_offset in 0..duration {
                let mut this_transform = ReanimTransform::default();

                if let Some(symbol) = &symbol_instance {
                    if let Some(lib_name) = symbol.attribute("libraryItemName") {
                        if temp_name != lib_name {
                            this_transform.i = Some(lib_name.to_string());
                            temp_name = lib_name.to_string();
                        }
                    }

                    if let Some(matrix_node) = symbol.children().find(|n| n.has_tag_name("matrix"))
                    {
                        if let Some(mat) = matrix_node.children().find(|n| n.has_tag_name("Matrix"))
                        {
                            let a = mat
                                .attribute("a")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(1.0);
                            let b = mat
                                .attribute("b")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let c = mat
                                .attribute("c")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let d = mat
                                .attribute("d")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(1.0);
                            let tx = mat
                                .attribute("tx")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let ty = mat
                                .attribute("ty")
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let skew_x = b.atan2(a);
                            let skew_y = c.atan2(d);
                            let quarter_pi = std::f64::consts::FRAC_PI_4;

                            let sx = if skew_x.abs() < quarter_pi || skew_x.abs() > 3.0 * quarter_pi
                            {
                                a / skew_x.cos()
                            } else {
                                b / skew_x.sin()
                            } as f32;

                            let sy = if skew_y.abs() < quarter_pi || skew_y.abs() > 3.0 * quarter_pi
                            {
                                d / skew_y.cos()
                            } else {
                                c / skew_y.sin()
                            } as f32;

                            let dx = 180.0 / std::f64::consts::PI;
                            let kx = (dx * skew_x) as f32;
                            let ky = (-dx * skew_y) as f32;

                            let tx = tx as f32;
                            let ty = ty as f32;

                            if (default_transform.x.unwrap_or(0.0) - tx).abs() > 0.001 {
                                default_transform.x = Some(tx);
                                this_transform.x = Some(tx);
                            }
                            if (default_transform.y.unwrap_or(0.0) - ty).abs() > 0.001 {
                                default_transform.y = Some(ty);
                                this_transform.y = Some(ty);
                            }
                            if (default_transform.kx.unwrap_or(0.0) - kx).abs() > 0.001 {
                                default_transform.kx = Some(kx);
                                this_transform.kx = Some(kx);
                            }
                            if (default_transform.ky.unwrap_or(0.0) - ky).abs() > 0.001 {
                                default_transform.ky = Some(ky);
                                this_transform.ky = Some(ky);
                            }
                            if (default_transform.sx.unwrap_or(1.0) - sx).abs() > 0.001 {
                                default_transform.sx = Some(sx);
                                this_transform.sx = Some(sx);
                            }
                            if (default_transform.sy.unwrap_or(1.0) - sy).abs() > 0.001 {
                                default_transform.sy = Some(sy);
                                this_transform.sy = Some(sy);
                            }

                            if frame_switch && frame_offset == 0 {
                                this_transform.f = Some(0.0);
                                frame_switch = false;
                            }
                        }
                    }

                    if let Some(color_node) = symbol.children().find(|n| n.has_tag_name("color")) {
                        if let Some(col) = color_node.children().find(|n| n.has_tag_name("Color")) {
                            let alpha = col
                                .attribute("alphaMultiplier")
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(1.0);
                            if (default_transform.a.unwrap_or(1.0) - alpha).abs() > 0.001 {
                                default_transform.a = Some(alpha);
                                this_transform.a = Some(alpha);
                            }
                        }
                    }

                    transforms.push(this_transform);
                } else {
                    if !frame_switch && frame_offset == 0 {
                        this_transform.f = Some(-1.0);
                        frame_switch = true;
                    }
                    transforms.push(this_transform);
                }
            }
        }

        tracks.push(ReanimTrack {
            name: layer_name,
            transforms,
        });
    }

    reanim.tracks = tracks;

    Ok(reanim)
}
