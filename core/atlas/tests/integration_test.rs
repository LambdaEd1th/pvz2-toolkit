use atlas::types::{OfficialAtlas, PathOrPaths, Resource};
use image::{ImageReader, Rgba, RgbaImage};
use std::fs;

#[test]
fn test_split_atlas_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let root = temp_dir.path();

    // 1. Create a 100x100 Image
    let mut img = RgbaImage::new(100, 100);
    // Fill 0,0 50x50 with Red
    for x in 0..50 {
        for y in 0..50 {
            img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }
    // Fill 50,50 50x50 with Blue
    for x in 50..100 {
        for y in 50..100 {
            img.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    let img_path = root.join("test_atlas.png");
    img.save(&img_path)?;

    // 2. Create JSON
    let atlas = OfficialAtlas {
        id: "test_atlas".to_string(),
        parent: "sprites".to_string(),
        res: "1200".to_string(),
        type_: "CompositeResourceGroup".to_string(),
        resources: vec![
            Resource {
                id: "red_rect".to_string(),
                type_: Some("Image".to_string()),
                path: Some(PathOrPaths::Single("red_rect.png".to_string())),
                width: Some(50),
                height: Some(50),
                ax: Some(0),
                ay: Some(0),
                aw: Some(50),
                ah: Some(50),
                x: Some(0),
                y: Some(0),
                cols: None,
                rows: None,
                atlas: Some(true),
            },
            Resource {
                id: "blue_rect".to_string(),
                type_: Some("Image".to_string()),
                path: Some(PathOrPaths::Single("blue_rect.png".to_string())),
                width: Some(50),
                height: Some(50),
                ax: Some(50),
                ay: Some(50),
                aw: Some(50),
                ah: Some(50),
                x: Some(0),
                y: Some(0),
                cols: None,
                rows: None,
                atlas: Some(true),
            },
        ],
    };
    let json_path = root.join("test_atlas.json");
    let json_str = serde_json::to_string(&atlas)?;
    fs::write(&json_path, json_str)?;

    // 3. Run Split
    atlas::split_atlas(&json_path, None, None)?;

    // 4. Verify Output
    // Default output: test_atlas.sprite/media
    let out_dir = root.join("test_atlas.sprite").join("media");
    assert!(out_dir.exists());

    let red_path = out_dir.join("red_rect.png");
    assert!(red_path.exists());
    let red_img = ImageReader::open(&red_path)?.decode()?.to_rgba8();
    assert_eq!(red_img.width(), 50);
    assert_eq!(red_img.height(), 50);
    assert_eq!(red_img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));

    let blue_path = out_dir.join("blue_rect.png");
    assert!(blue_path.exists());
    let blue_img = ImageReader::open(&blue_path)?.decode()?.to_rgba8();
    assert_eq!(blue_img.width(), 50);
    assert_eq!(blue_img.height(), 50);
    assert_eq!(blue_img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));

    Ok(())
}

#[test]
fn test_merge_atlas_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let root = temp_dir.path();

    // 1. Create a media directory with two loose sprites
    let media_dir = root.join("test_atlas.sprite").join("media");
    fs::create_dir_all(&media_dir)?;

    let mut red_img = RgbaImage::new(50, 50);
    for x in 0..50 {
        for y in 0..50 {
            red_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }
    red_img.save(media_dir.join("red_rect.png"))?;

    let mut blue_img = RgbaImage::new(50, 50);
    for x in 0..50 {
        for y in 0..50 {
            blue_img.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    blue_img.save(media_dir.join("blue_rect.png"))?;

    // 2. Create the JSON Descriptor with ax, ay unset
    let atlas = OfficialAtlas {
        id: "test_atlas".to_string(),
        parent: "sprites".to_string(),
        res: "1200".to_string(),
        type_: "CompositeResourceGroup".to_string(),
        resources: vec![
            Resource {
                id: "red_rect".to_string(),
                type_: Some("Image".to_string()),
                path: Some(PathOrPaths::Single("red_rect.png".to_string())),
                width: Some(50),
                height: Some(50),
                ax: None,
                ay: None,
                aw: None,
                ah: None,
                x: Some(0),
                y: Some(0),
                cols: None,
                rows: None,
                atlas: Some(true),
            },
            Resource {
                id: "blue_rect".to_string(),
                type_: Some("Image".to_string()),
                path: Some(PathOrPaths::Single("blue_rect.png".to_string())),
                width: Some(50),
                height: Some(50),
                ax: None,
                ay: None,
                aw: None,
                ah: None,
                x: Some(0),
                y: Some(0),
                cols: None,
                rows: None,
                atlas: Some(true),
            },
        ],
    };

    let json_path = root.join("test_atlas.json");
    let json_str = serde_json::to_string(&atlas)?;
    fs::write(&json_path, json_str)?;

    // 3. Run Merge
    atlas::merge_atlas(&json_path, None, None, None)?;

    // 4. Verify Output
    let out_img_path = root.join("test_atlas.png");
    assert!(out_img_path.exists());

    let out_json_path = root.join("test_atlas.json");
    let updated_json_content = fs::read_to_string(&out_json_path)?;
    let updated_atlas: OfficialAtlas = serde_json::from_str(&updated_json_content)?;

    // Both resources should now have mapped atlas coordinates
    for res in updated_atlas.resources {
        assert!(res.ax.is_some(), "Atlas X coord is missing");
        assert!(res.ay.is_some(), "Atlas Y coord is missing");
        assert!(res.aw.is_some(), "Atlas width is missing");
        assert!(res.ah.is_some(), "Atlas height is missing");
        assert_eq!(res.aw.unwrap(), 50);
        assert_eq!(res.ah.unwrap(), 50);
    }

    // Checking merged image dimensions
    let final_img = ImageReader::open(&out_img_path)?.decode()?.to_rgba8();
    // 50x50 packed together could be 128x64 or similar power-of-two depending on the algorithm
    assert!(
        final_img.width() >= 100
            || final_img.height() >= 100
            || (final_img.width() >= 64 && final_img.height() >= 64)
    );

    Ok(())
}
