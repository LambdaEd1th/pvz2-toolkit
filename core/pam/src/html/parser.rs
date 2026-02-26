use crate::types::PamInfo;
use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::fs;
use std::path::Path;

pub fn convert_to_html(pam: &PamInfo, output_path: &Path) -> Result<()> {
    // 1. Serialize PAM to JSON
    let pam_json = serde_json::to_string(pam).context("Failed to serialize PAM to JSON")?;

    // 2. Generate HTML content
    // We assume images are in a folder named 'media' relative to the HTML file?
    // Or we expect the user to put them there. The XFL converter created 'media'.
    // We will assume 'media/' prefix for images as consistent with XFL.

    let html_content = include_str!("template.html").replace("{pam_json}", &pam_json);

    fs::write(output_path, html_content)?;
    Ok(())
}

pub fn parse_html_pam(html_content: &str) -> Result<PamInfo> {
    let document = Html::parse_document(html_content);
    let selector = Selector::parse("script").unwrap();

    let mut found_json = String::new();

    for script in document.select(&selector) {
        let text = script.inner_html();
        let start_marker = "const pamData = ";
        if let Some(start_idx) = text.find(start_marker) {
            let start = start_idx + start_marker.len();
            let rest = &text[start..];

            // Extract JSON object by balancing braces
            let mut depth = 0;
            let mut end = 0;
            let mut found = false;
            for (i, c) in rest.char_indices() {
                if c == '{' {
                    depth += 1;
                    found = true;
                } else if c == '}' {
                    depth -= 1;
                    if found && depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
            }

            if end > 0 {
                found_json = rest[..end].to_string();
                break;
            }
        }
    }

    if found_json.is_empty() {
        anyhow::bail!("Could not extract JSON from HTML `<script>` tags");
    }

    let pam_info: PamInfo = serde_json::from_str(&found_json)
        .context("Failed to deserialize extracted JSON into PamInfo")?;
    Ok(pam_info)
}
