use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use image::{
    DynamicImage, GenericImage, GenericImageView, ImageDecoder, ImageReader, Rgba, RgbaImage,
    imageops,
};
use imageproc::drawing::{draw_text_mut, text_size};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

type AppResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextStyle {
    font_family: String,
    font_size: f32,
    color: String,
    horizontal_align: String,
    vertical_align: String,
    line_height: f32,
    letter_spacing: f32,
    padding: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearStrategy {
    mode: String,
    patch_path: Option<String>,
    inpaint_threshold: Option<u8>,
    inpaint_radius: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateField {
    id: String,
    name: String,
    kind: String,
    value: String,
    csv_column: Option<String>,
    date_value: Option<String>,
    time_value: Option<String>,
    date_format: Option<String>,
    random_min: Option<f64>,
    random_max: Option<f64>,
    random_decimals: Option<u32>,
    erase_rect: Rect,
    layout_rect: Rect,
    text: TextStyle,
    clear: ClearStrategy,
    enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Background {
    path: String,
    width: u32,
    height: u32,
    sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateDocument {
    schema_version: u32,
    id: String,
    name: String,
    background: Background,
    fields: Vec<TemplateField>,
    created_at: String,
    updated_at: String,
}
#[derive(Serialize)]
struct ImageMeta {
    width: u32,
    height: u32,
}

#[tauri::command]
fn inspect_image(path: String) -> AppResult<ImageMeta> {
    let supported = Path::new(&path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or(false);
    if !supported {
        return Err("仅支持 PNG、JPG 或 JPEG 图片".into());
    }
    let img = open_oriented_image(&path)?;
    Ok(ImageMeta {
        width: img.width(),
        height: img.height(),
    })
}

#[tauri::command]
fn load_template(path: String) -> AppResult<TemplateDocument> {
    let raw = fs::read_to_string(path).map_err(|e| format!("读取模板失败：{e}"))?;
    let doc: TemplateDocument =
        serde_json::from_str(&raw).map_err(|e| format!("模板格式错误：{e}"))?;
    if doc.schema_version != 1 {
        return Err(format!("不支持的模板版本：{}", doc.schema_version));
    }
    Ok(doc)
}

#[tauri::command]
fn save_template(path: String, template: TemplateDocument) -> AppResult<()> {
    let json = serde_json::to_string_pretty(&template).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("保存模板失败：{e}"))
}

#[tauri::command]
fn render_inpaint_patch(
    background_path: String,
    rect: Rect,
    threshold: Option<u8>,
    radius: Option<u32>,
) -> AppResult<Vec<u8>> {
    let pristine = open_oriented_image(&background_path)?.to_rgba8();
    let rect = clipped(&rect, pristine.width(), pristine.height());
    if rect.width == 0 || rect.height == 0 {
        return Err("智能抹除区域超出图片范围".into());
    }
    let mut result = pristine.clone();
    inpaint_text(
        &mut result,
        &pristine,
        &rect,
        threshold.unwrap_or(14),
        radius.unwrap_or(2).min(6),
    );
    let patch = result
        .view(rect.x, rect.y, rect.width, rect.height)
        .to_image();
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(patch)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| format!("生成智能抹除预览失败：{e}"))?;
    Ok(bytes.into_inner())
}

#[tauri::command]
fn render_text_patch(mut field: TemplateField, value: String) -> AppResult<Vec<u8>> {
    let width = field.layout_rect.width.max(1);
    let height = field.layout_rect.height.max(1);
    field.layout_rect = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let mut patch = RgbaImage::new(width, height);
    draw_field(&mut patch, &field, &value)?;
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(patch)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| format!("生成文字图层失败：{e}"))?;
    Ok(bytes.into_inner())
}

#[tauri::command]
fn render_preview(
    template: TemplateDocument,
    csv_path: Option<String>,
    output_format: String,
) -> AppResult<Vec<u8>> {
    let row = match csv_path.filter(|path| !path.is_empty()) {
        Some(path) => read_csv(&path)?.into_iter().next().unwrap_or_default(),
        None => HashMap::new(),
    };
    let canvas = render_template_row(&template, &row)?;
    let mut bytes = Cursor::new(Vec::new());
    if output_format.eq_ignore_ascii_case("png") {
        DynamicImage::ImageRgba8(canvas)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| format!("生成 PNG 预览失败：{e}"))?;
    } else if matches!(output_format.to_ascii_lowercase().as_str(), "jpg" | "jpeg") {
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(canvas).to_rgb8())
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .map_err(|e| format!("生成 JPG 预览失败：{e}"))?;
    } else {
        return Err(format!("不支持的预览格式：{output_format}"));
    }
    Ok(bytes.into_inner())
}

#[tauri::command]
fn generate_batch(
    template: TemplateDocument,
    csv_path: Option<String>,
    output_dir: String,
    output_format: String,
) -> AppResult<usize> {
    let output_format = output_format.to_ascii_lowercase();
    if !matches!(output_format.as_str(), "png" | "jpg" | "jpeg") {
        return Err(format!("不支持的输出格式：{output_format}"));
    }
    fs::create_dir_all(&output_dir).map_err(|e| format!("无法创建输出目录：{e}"))?;
    let rows = match csv_path.filter(|s| !s.is_empty()) {
        Some(path) => read_csv(&path)?,
        None => vec![HashMap::new()],
    };
    for (index, row) in rows.iter().enumerate() {
        let canvas = render_template_row(&template, row)?;
        let extension = if output_format == "png" { "png" } else { "jpg" };
        let file_name = format!(
            "{}_{:04}.{}",
            safe_name(&template.name),
            index + 1,
            extension
        );
        let output_path = Path::new(&output_dir).join(file_name);
        if extension == "png" {
            canvas
                .save(output_path)
                .map_err(|e| format!("写入 PNG 失败：{e}"))?;
        } else {
            image::DynamicImage::ImageRgba8(canvas)
                .to_rgb8()
                .save(output_path)
                .map_err(|e| format!("写入 JPG 失败：{e}"))?;
        }
    }
    Ok(rows.len())
}

fn render_template_row(
    template: &TemplateDocument,
    row: &HashMap<String, String>,
) -> AppResult<RgbaImage> {
    let mut canvas = open_oriented_image(&template.background.path)?.to_rgba8();
    let pristine = canvas.clone();
    for field in template.fields.iter().filter(|field| field.enabled) {
        clear_region(&mut canvas, &pristine, field)?;
        let value = resolve_value(field, row);
        draw_field(&mut canvas, field, &value)?;
    }
    Ok(canvas)
}

fn read_csv(path: &str) -> AppResult<Vec<HashMap<String, String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("打开 CSV 失败：{e}"))?;
    let headers = reader
        .headers()
        .map_err(|e| format!("CSV 表头错误：{e}"))?
        .clone();
    reader
        .records()
        .map(|record| {
            let record = record.map_err(|e| format!("CSV 数据错误：{e}"))?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect())
        })
        .collect()
}

/// Decode once and apply EXIF orientation before dimensions or pixels are used.
/// Browser image elements display JPEG orientation automatically; doing the same
/// here keeps editor coordinates and exported pixels in exactly the same space.
fn open_oriented_image(path: &str) -> AppResult<DynamicImage> {
    let reader = ImageReader::open(path)
        .map_err(|e| format!("无法打开图片：{e}"))?
        .with_guessed_format()
        .map_err(|e| format!("无法识别图片格式：{e}"))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("无法创建图片解码器：{e}"))?;
    let orientation = decoder
        .orientation()
        .map_err(|e| format!("无法读取图片方向：{e}"))?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|e| format!("无法解码图片：{e}"))?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn clear_region(
    canvas: &mut RgbaImage,
    pristine: &RgbaImage,
    field: &TemplateField,
) -> AppResult<()> {
    let r = clipped(&field.erase_rect, canvas.width(), canvas.height());
    if r.width == 0 || r.height == 0 {
        return Ok(());
    }
    match field.clear.mode.as_str() {
        "inpaint" => inpaint_text(
            canvas,
            pristine,
            &r,
            field.clear.inpaint_threshold.unwrap_or(14),
            field.clear.inpaint_radius.unwrap_or(2).min(6),
        ),
        "patch" => {
            let path = field
                .clear
                .patch_path
                .as_ref()
                .ok_or_else(|| format!("字段“{}”未配置背景补丁", field.name))?;
            let patch = image::open(path)
                .map_err(|e| format!("读取补丁失败：{e}"))?
                .resize_exact(r.width, r.height, imageops::FilterType::Lanczos3)
                .to_rgba8();
            canvas
                .copy_from(&patch, r.x, r.y)
                .map_err(|e| e.to_string())?
        }
        mode => return Err(format!("未知清除模式：{mode}")),
    }
    Ok(())
}

/// Remove only dark foreground strokes inside the erase rectangle.
///
/// A Gaussian local-background estimate separates printed text from paper
/// texture. The mask is slightly dilated to include antialiasing, then filled
/// inward from untouched neighboring pixels. Pixels outside the detected mask
/// are copied verbatim and therefore retain the original scan texture.
fn inpaint_text(
    canvas: &mut RgbaImage,
    pristine: &RgbaImage,
    rect: &Rect,
    threshold: u8,
    radius: u32,
) {
    let source = pristine
        .view(rect.x, rect.y, rect.width, rect.height)
        .to_image();
    let background = imageops::blur(&source, 3.0);
    let width = rect.width as usize;
    let height = rect.height as usize;
    let mut mask = vec![false; width * height];

    for y in 0..height {
        for x in 0..width {
            let original = source.get_pixel(x as u32, y as u32);
            let estimated = background.get_pixel(x as u32, y as u32);
            mask[y * width + x] =
                luminance(estimated).saturating_sub(luminance(original)) >= threshold;
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
                        if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
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
                if count > 0 {
                    frontier.push((
                        index,
                        x as u32,
                        y as u32,
                        Rgba([
                            (sum[0] / count) as u8,
                            (sum[1] / count) as u8,
                            (sum[2] / count) as u8,
                            (sum[3] / count) as u8,
                        ]),
                    ));
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

    let _ = canvas.copy_from(&result, rect.x, rect.y);
}

fn luminance(pixel: &Rgba<u8>) -> u8 {
    ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000) as u8
}

fn draw_field(canvas: &mut RgbaImage, field: &TemplateField, value: &str) -> AppResult<()> {
    if value.is_empty() {
        return Ok(());
    }
    let font = load_font(&field.text)?;
    let scale = PxScale::from(field.text.font_size.max(1.0));
    let (tw, _) = text_size(scale, &font, value);
    let (glyph_min_y, glyph_max_y) = glyph_vertical_bounds(&font, scale, value);
    let r = &field.layout_rect;
    let pad = field.text.padding;
    let inner_w = r.width.saturating_sub(pad * 2);
    let x = match field.text.horizontal_align.as_str() {
        "right" => r.x + pad + inner_w.saturating_sub(tw),
        "center" => r.x + pad + inner_w.saturating_sub(tw) / 2,
        _ => r.x + pad,
    };
    let line_height = field.text.font_size.max(1.0) * field.text.line_height.max(0.1);
    let line_top = line_box_top(
        r.y as f32,
        r.height as f32,
        pad as f32,
        line_height,
        &field.text.vertical_align,
    );
    // imageproc positions glyphs relative to the font baseline and adds px_bounds.min.y
    // during drawing. Offset by the actual glyph-box center so it matches Konva's
    // middle-baseline line box instead of being shifted down by the ascent.
    let target_center = line_top + line_height / 2.0;
    let glyph_center = (glyph_min_y + glyph_max_y) / 2.0;
    let y = (target_center - glyph_center).round() as i32;
    draw_text_mut(
        canvas,
        parse_color(&field.text.color)?,
        x as i32,
        y,
        scale,
        &font,
        value,
    );
    Ok(())
}

fn glyph_vertical_bounds(font: &FontArc, scale: PxScale, text: &str) -> (f32, f32) {
    let scaled = font.as_scaled(scale);
    let mut caret = 0.0;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for character in text.chars() {
        let id = scaled.glyph_id(character);
        let glyph = id.with_scale_and_position(scale, point(caret, scaled.ascent()));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            min_y = min_y.min(bounds.min.y);
            max_y = max_y.max(bounds.max.y);
        }
        caret += scaled.h_advance(id);
    }
    if min_y.is_finite() && max_y.is_finite() {
        (min_y, max_y)
    } else {
        (0.0, scale.y)
    }
}

fn line_box_top(y: f32, height: f32, padding: f32, line_height: f32, align: &str) -> f32 {
    let content_height = height - padding * 2.0;
    let offset = match align {
        "middle" => (content_height - line_height) / 2.0,
        "bottom" => content_height - line_height,
        _ => 0.0,
    };
    y + padding + offset
}

fn load_font(style: &TextStyle) -> AppResult<FontArc> {
    let paths: &[&str] = match style.font_family.as_str() {
        "宋体" => &[
            "/System/Library/Fonts/Supplemental/Songti.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        ],
        "仿宋" => &[
            "/System/Library/Fonts/STFangsong.ttf",
            "/System/Library/Fonts/Supplemental/STFangsong.ttf",
            "/System/Library/Fonts/Supplemental/Songti.ttc",
            "C:\\Windows\\Fonts\\simfang.ttf",
            "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        ],
        "Arial" => &[
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        ],
        "Times New Roman" => &[
            "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
            "C:\\Windows\\Fonts\\times.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSerif-Regular.ttf",
        ],
        _ => &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ],
    };
    let candidates: Vec<PathBuf> = paths.iter().map(|path| PathBuf::from(*path)).collect();
    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Ok(font);
            }
        }
    }
    Err(format!("当前系统没有安装字体“{}”", style.font_family))
}

fn resolve_value(field: &TemplateField, row: &HashMap<String, String>) -> String {
    match field.kind.as_str() {
        "csv" => field
            .csv_column
            .as_ref()
            .and_then(|k| row.get(k))
            .cloned()
            .unwrap_or_default(),
        "date" => {
            let legacy = field.value.replace(' ', "T");
            let (legacy_date, legacy_time) = legacy.split_once('T').unwrap_or((&legacy, "00:00"));
            let date = field
                .date_value
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(legacy_date);
            let time = field
                .time_value
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(legacy_time);
            format_date_time(
                &format!("{date}T{time}"),
                field.date_format.as_deref().unwrap_or("YYYY-MM-DD HH:mm"),
            )
        }
        "random" => {
            let first = field.random_min.unwrap_or(0.0);
            let second = field.random_max.unwrap_or(100.0);
            let (min, max) = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            let value = if (max - min).abs() < f64::EPSILON {
                min
            } else {
                rand::rng().random_range(min..=max)
            };
            format!(
                "{:.*}",
                field.random_decimals.unwrap_or(0).min(8) as usize,
                value
            )
        }
        _ => field.value.clone(),
    }
}

fn format_date_time(value: &str, pattern: &str) -> String {
    let normalized = value.replace('T', " ");
    let mut parts = normalized.split(['-', ' ', ':']);
    let Some(year) = parts.next() else {
        return value.to_owned();
    };
    let Some(month) = parts.next() else {
        return value.to_owned();
    };
    let Some(day) = parts.next() else {
        return value.to_owned();
    };
    let Some(hour) = parts.next() else {
        return value.to_owned();
    };
    let Some(minute) = parts.next() else {
        return value.to_owned();
    };
    let pattern = if pattern.is_empty() {
        "YYYY-MM-DD HH:mm"
    } else {
        pattern
    };
    pattern
        .replace("YYYY", year)
        .replace("MM", month)
        .replace("DD", day)
        .replace("HH", hour)
        .replace("mm", minute)
}

fn clipped(r: &Rect, width: u32, height: u32) -> Rect {
    Rect {
        x: r.x.min(width),
        y: r.y.min(height),
        width: r.width.min(width.saturating_sub(r.x)),
        height: r.height.min(height.saturating_sub(r.y)),
    }
}
fn parse_color(s: &str) -> AppResult<Rgba<u8>> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 && h.len() != 8 {
        return Err(format!("无效颜色：{s}"));
    }
    let v = u32::from_str_radix(h, 16).map_err(|_| format!("无效颜色：{s}"))?;
    Ok(if h.len() == 6 {
        Rgba([
            ((v >> 16) & 255) as u8,
            ((v >> 8) & 255) as u8,
            (v & 255) as u8,
            255,
        ])
    } else {
        Rgba([
            ((v >> 24) & 255) as u8,
            ((v >> 16) & 255) as u8,
            ((v >> 8) & 255) as u8,
            (v & 255) as u8,
        ])
    })
}
fn safe_name(s: &str) -> String {
    s.chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            inspect_image,
            load_template,
            save_template,
            render_inpaint_patch,
            render_text_patch,
            render_preview,
            generate_batch
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inpaint_removes_dark_strokes_without_touching_other_pixels() {
        let mut source = RgbaImage::from_pixel(80, 30, Rgba([205, 204, 202, 255]));
        // Add deterministic scan-like texture.
        for y in 0..30 {
            for x in 0..80 {
                let noise = ((x * 7 + y * 3) % 5) as u8;
                source.put_pixel(x, y, Rgba([205 - noise, 204 - noise, 202 - noise, 255]));
            }
        }
        // A narrow dark glyph stroke inside the erase rectangle.
        for y in 8..23 {
            for x in 38..41 {
                source.put_pixel(x, y, Rgba([25, 25, 24, 255]));
            }
        }
        let pristine = source.clone();
        let untouched = *source.get_pixel(24, 10);
        let mut result = source;
        inpaint_text(
            &mut result,
            &pristine,
            &Rect {
                x: 20,
                y: 5,
                width: 40,
                height: 22,
            },
            14,
            2,
        );

        assert_eq!(*result.get_pixel(24, 10), untouched);
        assert!(luminance(result.get_pixel(39, 15)) > 150);
        assert_eq!(*result.get_pixel(5, 5), *pristine.get_pixel(5, 5));
    }

    #[test]
    fn vertical_alignment_keeps_anchors_when_font_exceeds_the_box() {
        let y = 100.0;
        let height = 40.0;
        let padding = 4.0;
        for line_height in [12.0, 40.0, 80.0, 160.0] {
            let top = line_box_top(y, height, padding, line_height, "top");
            let middle = line_box_top(y, height, padding, line_height, "middle");
            let bottom = line_box_top(y, height, padding, line_height, "bottom");
            assert_eq!(top, y + padding);
            assert_eq!(middle + line_height / 2.0, y + height / 2.0);
            assert_eq!(bottom + line_height, y + height - padding);
        }
    }

    #[test]
    fn date_time_supports_minute_precision_and_custom_formats() {
        assert_eq!(
            format_date_time("2026-05-11T09:54", "YYYY-MM-DD HH:mm"),
            "2026-05-11 09:54"
        );
        assert_eq!(
            format_date_time("2026-05-11T09:54", "YYYY年MM月DD日 HH时mm分"),
            "2026年05月11日 09时54分"
        );
    }
}
