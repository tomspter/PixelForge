use image::{GenericImageView, Rgba, RgbaImage, imageops};

use super::{OrientedRegion};

/// Legacy dark-stroke remover.
///
/// A local background estimate produces a sparse text mask. The mask is
/// dilated to include antialiasing and then filled inward one pixel at a time.
pub(crate) fn apply(
    canvas: &mut RgbaImage,
    pristine: &RgbaImage,
    region: &OrientedRegion,
    threshold: u8,
    radius: u32,
) {
    let bounds = &region.bounds;
    let source = pristine
        .view(bounds.x, bounds.y, bounds.width, bounds.height)
        .to_image();
    let background = imageops::blur(&source, 3.0);
    let width = bounds.width as usize;
    let height = bounds.height as usize;
    let mut mask = vec![false; width * height];

    for y in 0..height {
        for x in 0..width {
            let original = source.get_pixel(x as u32, y as u32);
            let estimated = background.get_pixel(x as u32, y as u32);
            mask[y * width + x] = region.contains(bounds.x + x as u32, bounds.y + y as u32)
                && luminance(estimated).saturating_sub(luminance(original)) >= threshold;
        }
    }

    if radius > 0 {
        let original_mask = mask.clone();
        let radius = radius as i32;
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if !original_mask[y as usize * width + x as usize] {
                    continue;
                }
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32
                            && region.contains(bounds.x + nx as u32, bounds.y + ny as u32)
                        {
                            mask[ny as usize * width + nx as usize] = true;
                        }
                    }
                }
            }
        }
    }

    let mut result = source.clone();
    let mut known: Vec<bool> = mask.iter().map(|masked| !masked).collect();
    let mut remaining = mask.iter().filter(|masked| **masked).count();

    while remaining > 0 {
        let mut frontier = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;
                if known[index] {
                    continue;
                }
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let neighbor = ny as usize * width + nx as usize;
                        if known[neighbor] {
                            let pixel = result.get_pixel(nx as u32, ny as u32);
                            for channel in 0..4 {
                                sum[channel] += pixel[channel] as u32;
                            }
                            count += 1;
                        }
                    }
                }
                let averaged =
                    sum.map(|channel| channel.checked_div(count).unwrap_or_default() as u8);
                if count > 0 {
                    frontier.push((index, x as u32, y as u32, Rgba(averaged)));
                }
            }
        }
        if frontier.is_empty() {
            break;
        }
        for (index, x, y, pixel) in frontier {
            result.put_pixel(x, y, pixel);
            known[index] = true;
            remaining -= 1;
        }
    }

    for y in 0..bounds.height {
        for x in 0..bounds.width {
            if mask[y as usize * width + x as usize] {
                canvas.put_pixel(bounds.x + x, bounds.y + y, *result.get_pixel(x, y));
            }
        }
    }
}

fn luminance(pixel: &Rgba<u8>) -> u8 {
    ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000) as u8
}

#[cfg(test)]
mod tests {
    use crate::Rect;
use super::*;

    #[test]
    fn removes_dark_strokes_without_touching_other_pixels() {
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
        let untouched = *source.get_pixel(24, 10);
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
            2,
        );

        assert_eq!(*result.get_pixel(24, 10), untouched);
        assert!(luminance(result.get_pixel(39, 15)) > 150);
        assert_eq!(*result.get_pixel(5, 5), *pristine.get_pixel(5, 5));
    }
}
