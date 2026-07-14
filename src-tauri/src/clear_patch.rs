use image::{RgbaImage, imageops};

use super::{AppResult, OrientedRegion, composite_region};

/// Replace the erase rectangle with a user-provided image patch.
pub(crate) fn apply(
    canvas: &mut RgbaImage,
    region: &OrientedRegion,
    patch_path: Option<&str>,
    field_name: &str,
) -> AppResult<()> {
    let patch = load(patch_path, field_name, region)?;
    composite_region(canvas, &patch, region);
    Ok(())
}

pub(crate) fn load(
    patch_path: Option<&str>,
    field_name: &str,
    region: &OrientedRegion,
) -> AppResult<RgbaImage> {
    let path = patch_path.ok_or_else(|| format!("字段“{field_name}”尚未选择背景补丁图片"))?;
    let patch = image::open(path)
        .map_err(|_| {
            format!("无法读取字段“{field_name}”的背景补丁，请确认文件仍然存在且格式有效")
        })?
        .resize_exact(region.rect.width, region.rect.height, imageops::FilterType::Lanczos3)
        .to_rgba8();
    Ok(patch)
}
