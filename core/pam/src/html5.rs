use crate::PamInfo;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn convert_to_html(pam: &PamInfo, output_path: &Path) -> Result<()> {
    // 1. Serialize PAM to JSON
    let pam_json = serde_json::to_string(pam).context("Failed to serialize PAM to JSON")?;

    // 2. Generate HTML content
    // We assume images are in a folder named 'media' relative to the HTML file?
    // Or we expect the user to put them there. The XFL converter created 'media'.
    // We will assume 'media/' prefix for images as consistent with XFL.

    let html_content = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PAM Animation Preview</title>
    <style>
        body {{ margin: 0; background-color: #333; display: flex; justify-content: center; align-items: center; height: 100vh; color: white; font-family: sans-serif; }}
        canvas {{ border: 1px solid #666; background-image: linear-gradient(45deg, #444 25%, transparent 25%), linear-gradient(-45deg, #444 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #444 75%), linear-gradient(-45deg, transparent 75%, #444 75%); background-size: 20px 20px; background-position: 0 0, 0 10px, 10px -10px, -10px 0px; }}
        #controls {{ position: absolute; bottom: 20px; background: rgba(0,0,0,0.7); padding: 10px; border-radius: 8px; }}
    </style>
</head>
<body>
    <canvas id="pamCanvas"></canvas>
    <div id="controls">
        <button onclick="togglePlay()">Play/Pause</button>
        <span id="frameDisplay">Frame: 0</span>
    </div>

    <script>
        const pamData = {pam_json};
        
        // --- Runtime Engine ---
        
        const canvas = document.getElementById('pamCanvas');
        const ctx = canvas.getContext('2d');
        const frameDisplay = document.getElementById('frameDisplay');
        
        // Normalize size
        canvas.width = pamData.size[0];
        canvas.height = pamData.size[1];
        
        let isPlaying = true;
        let images = [];
        let sprites = pamData.sprite;
        let mainSprite = pamData.main_sprite;
        
        // Preload images
        let loadedCount = 0;
        
        function preloadImages() {{
            pamData.image.forEach((imgInfo, index) => {{
                const img = new Image();
                // Fix name to match XFL extraction: 'media/NAME.png'
                // imgInfo.name might be "IMAGE_..." or "Group|Name"
                let name = imgInfo.name;
                if (name.includes('|')) {{
                    name = name.split('|')[0];
                }}
                img.src = 'media/' + name + '.png'; 
                img.onload = () => {{
                    loadedCount++;
                    if (loadedCount === pamData.image.length) {{
                        startAnimation();
                    }}
                }};
                img.onerror = () => {{
                     console.warn("Failed to load image:", img.src);
                     // Count it anyway to avoid stall
                     loadedCount++;
                     if (loadedCount === pamData.image.length) {{
                        startAnimation();
                    }}
                }};
                images[index] = img;
            }});
        }}
        
        // --- Classes ---
        
        class Transform {{
            constructor(a, b, c, d, tx, ty) {{
                this.a = a; this.b = b; this.c = c; this.d = d; 
                this.tx = tx; this.ty = ty;
            }}
            
            static identity() {{
                return new Transform(1, 0, 0, 1, 0, 0);
            }}
            
            static fromArray(arr) {{
                if (arr.length === 6) return new Transform(arr[0], arr[1], arr[2], arr[3], arr[4], arr[5]);
                // Simplified fallbacks
                if (arr.length === 2) return new Transform(1, 0, 0, 1, arr[0], arr[1]); 
                return Transform.identity();
            }}
            
            multiply(other) {{
                return new Transform(
                    this.a * other.a + this.b * other.c,
                    this.a * other.b + this.b * other.d,
                    this.c * other.a + this.d * other.c,
                    this.c * other.b + this.d * other.d,
                    this.a * other.tx + this.b * other.ty + this.tx,
                    this.a * other.ty + this.b * other.ty + this.ty 
                );
            }}
        }}

        class SpriteInstance {{
            constructor() {{
                this.currentFrame = 0;
                this.layers = new Map(); // layerIndex -> {{ resource, isSprite, transform, color, instance? }}
            }}
            
            updateAndDraw(ctx, spriteData, parentTransform, alpha) {{
                // 1. Process Timeline for Current Frame
                if (this.currentFrame < spriteData.frame.length) {{
                    const frameInfo = spriteData.frame[this.currentFrame];
                    
                    // Remove
                    if (frameInfo.remove) {{
                        frameInfo.remove.forEach(cmd => this.layers.delete(cmd.index));
                    }}
                    
                    // Append
                    if (frameInfo.append) {{
                        frameInfo.append.forEach(cmd => {{
                            let instance = null;
                            if (cmd.sprite) {{
                                // Create new sprite instance
                                // Which sprite? pamData.sprite[cmd.resource]
                                // Caution: Recursive depth check?
                                instance = new SpriteInstance();
                            }}
                            this.layers.set(cmd.index, {{
                                resource: cmd.resource,
                                isSprite: cmd.sprite,
                                transform: Transform.identity(),
                                color: [1,1,1,1],
                                instance: instance
                            }});
                        }});
                    }}
                    
                    // Change
                    if (frameInfo.change) {{
                        frameInfo.change.forEach(cmd => {{
                            const layer = this.layers.get(cmd.index);
                            if (layer) {{
                                if (cmd.transform) layer.transform = Transform.fromArray(cmd.transform);
                                if (cmd.color) layer.color = cmd.color;
                            }}
                        }});
                    }}
                }}
                
                // 2. Draw Layers (Sorted by depth/index descending?)
                const sortedKeys = Array.from(this.layers.keys()).sort((a, b) => a - b);
                
                for (const key of sortedKeys) {{
                    const layer = this.layers.get(key);
                    
                    // Combine Transform
                    ctx.save();
                    
                    const t = layer.transform;
                    ctx.transform(t.a, t.b, t.c, t.d, t.tx, t.ty);
                    
                    const localAlpha = alpha * (layer.color ? layer.color[3] : 1.0);
                    ctx.globalAlpha = localAlpha;
                    
                    if (layer.isSprite) {{
                        // Recurse
                        const subSpriteData = sprites[layer.resource];
                        if (subSpriteData && layer.instance) {{
                            layer.instance.updateAndDraw(ctx, subSpriteData, null, localAlpha);
                        }}
                    }} else {{
                        // Draw Image
                        const img = images[layer.resource];
                        const imgInfo = pamData.image[layer.resource];
                        if (img && imgInfo) {{
                            const it = Transform.fromArray(imgInfo.transform);
                            ctx.transform(it.a, it.b, it.c, it.d, it.tx, it.ty);
                            ctx.drawImage(img, 0, 0); 
                        }}
                    }}
                    
                    ctx.restore();
                }}
                
                // Advance frame
                this.currentFrame++;
                // Loop main sprite
                if (this.currentFrame >= spriteData.frame.length) this.currentFrame = 0; 
            }}
        }}
        
        let rootInstance = null;
        
        function startAnimation() {{
            rootInstance = new SpriteInstance();
            requestAnimationFrame(draw);
        }}
        
        function draw() {{
            if (!isPlaying) return;
            
            ctx.setTransform(1, 0, 0, 1, 0, 0);
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            
            ctx.translate(pamData.position[0], pamData.position[1]);
            
            if (rootInstance) {{
                // Use main_sprite data
                rootInstance.updateAndDraw(ctx, mainSprite, null, 1.0);
            }}
            
            frameDisplay.innerText = "Frame: " + (rootInstance ? rootInstance.currentFrame : 0);
            
            setTimeout(() => requestAnimationFrame(draw), 1000 / pamData.frame_rate);
        }}
        
        function togglePlay() {{
            isPlaying = !isPlaying;
            if (isPlaying) draw();
        }}
        
        if (pamData.image.length === 0) {{
             startAnimation();
        }} else {{
             preloadImages();
        }}
        
    </script>
</body>
</html>
"#,
        pam_json = pam_json
    );

    fs::write(output_path, html_content)?;
    Ok(())
}
