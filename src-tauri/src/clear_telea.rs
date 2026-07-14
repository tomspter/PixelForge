use image::{GenericImageView, GrayImage, Luma, Rgba, RgbaImage, imageops};
use inpaint::prelude::ImageInpaint;

use super::{AppResult, OrientedRegion, Rect};

/// Detect dark text independently and repair only those sparse mask pixels
/// with the Rust `inpaint` crate's Telea implementation.
pub(crate) fn apply(
    canvas: &mut RgbaImage,
    pristine: &RgbaImage,
    region: &OrientedRegion,
    threshold: u8,
    mask_radius: u32,
    radius: u32,
) -> AppResult<()> {
    validate_settings(threshold, mask_radius, radius)?;
    let context = expanded(
        &region.bounds,
        radius.saturating_mul(2).max(mask_radius.saturating_add(6)),
        pristine.width(),
        pristine.height(),
    );
    let mut result = pristine
        .view(context.x, context.y, context.width, context.height)
        .to_image();
    let background = imageops::blur(&result, 3.0);
    let local_x = region.bounds.x - context.x;
    let local_y = region.bounds.y - context.y;
    let mut detected = vec![false; context.width as usize * context.height as usize];

    for y in local_y..local_y + region.bounds.height {
        for x in local_x..local_x + region.bounds.width {
            let original = result.get_pixel(x, y);
            let estimated = background.get_pixel(x, y);
            detected[y as usize * context.width as usize + x as usize] = region.contains(context.x + x, context.y + y)
                && luminance(estimated).saturating_sub(luminance(original)) >= threshold;
        }
    }

    dilate_mask(
        &mut detected,
        context.width,
        local_x,
        local_y,
        region.bounds.width,
        region.bounds.height,
        mask_radius,
        region,
        context.x,
        context.y,
    );
    if !detected.iter().any(|masked| *masked) {
        return Ok(());
    }

    let mask = GrayImage::from_fn(context.width, context.height, |x, y| {
        Luma([
            if detected[y as usize * context.width as usize + x as usize] {
                u8::MAX
            } else {
                0
            },
        ])
    });
    result
        .telea_inpaint(&mask, radius as i32)
        .map_err(|_| "Telea 修复失败，请减小修复区域或半径后重试".to_owned())?;
    let patch = result
        .view(local_x, local_y, region.bounds.width, region.bounds.height)
        .to_image();
    for y in 0..region.bounds.height {
        for x in 0..region.bounds.width {
            let context_index = (local_y + y) as usize * context.width as usize + (local_x + x) as usize;
            if detected[context_index] {
                canvas.put_pixel(region.bounds.x + x, region.bounds.y + y, *patch.get_pixel(x, y));
            }
        }
    }
    Ok(())
}

fn validate_settings(threshold: u8, mask_radius: u32, radius: u32) -> AppResult<()> {
    if !(1..=80).contains(&threshold) {
        return Err("Telea 文字阈值必须在 1 到 80 之间".into());
    }
    if mask_radius > 12 {
        return Err("Telea 掩码扩张必须在 0 到 12 之间".into());
    }
    if !(1..=100).contains(&radius) {
        return Err("Telea 修复半径必须在 1 到 100 之间".into());
    }
    Ok(())
}

fn dilate_mask(
    mask: &mut [bool],
    stride: u32,
    rect_x: u32,
    rect_y: u32,
    rect_width: u32,
    rect_height: u32,
    radius: u32,
    region: &OrientedRegion,
    context_x: u32,
    context_y: u32,
) {
    if radius == 0 {
        return;
    }
    let original = mask.to_vec();
    let radius = radius as i32;
    let left = rect_x as i32;
    let top = rect_y as i32;
    let right = left + rect_width as i32;
    let bottom = top + rect_height as i32;
    for y in top..bottom {
        for x in left..right {
            if !original[y as usize * stride as usize + x as usize] {
                continue;
            }
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= left && nx < right && ny >= top && ny < bottom
                        && region.contains(context_x + nx as u32, context_y + ny as u32)
                    {
                        mask[ny as usize * stride as usize + nx as usize] = true;
                    }
                }
            }
        }
    }
}

fn expanded(rect: &Rect, padding: u32, width: u32, height: u32) -> Rect {
    let x = rect.x.saturating_sub(padding);
    let y = rect.y.saturating_sub(padding);
    let right = rect
        .x
        .saturating_add(rect.width)
        .saturating_add(padding)
        .min(width);
    let bottom = rect
        .y
        .saturating_add(rect.height)
        .saturating_add(padding)
        .min(height);
    Rect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    }
}

fn luminance(pixel: &Rgba<u8>) -> u8 {
    ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_only_detected_text_and_keeps_unmasked_background() {
        let mut source = RgbaImage::from_pixel(80, 30, Rgba([205, 204, 202, 255]));
        for y in 0..30 {
            for x in 0..80 {
                let noise = ((x * 7 + y * 3) % 5) as u8;
                source.put_pixel(x, y, Rgba([205 - noise, 204 - noise, 202 - noise, 255]));
            }
        }
        for y in 8..23 {
            for x in 38..41 {
                source.put_pixel(x, y, Rgba([25, 25, 24, 255]));
            }
        }
        let pristine = source.clone();
        let untouched_inside_rect = *source.get_pixel(24, 10);
        let mut result = source;
        apply(
            &mut result,
            &pristine,
            &OrientedRegion::new(&Rect {
                x: 20,
                y: 5,
                width: 40,
                height: 22,
            }, (40.0, 16.0), 0.0, 80, 30),
            14,
            1,
            3,
        )
        .unwrap();

        assert_eq!(*result.get_pixel(24, 10), untouched_inside_rect);
        assert!(luminance(result.get_pixel(39, 15)) > 150);
        assert_eq!(*result.get_pixel(5, 5), *pristine.get_pixel(5, 5));
        assert!(validate_settings(14, 1, 37).is_ok());
        assert!(validate_settings(14, 1, 0).is_err());
        assert!(validate_settings(14, 13, 3).is_err());
    }
}
