mod clear_inpaint;
mod clear_patch;
mod clear_telea;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageReader, Rgba, RgbaImage};
use imageproc::drawing::{draw_text_mut, text_size};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, ErrorKind},
    path::{Path, PathBuf},
};

type AppResult<T> = Result<T, String>;

fn io_error_message(action: &str, error: &std::io::Error) -> String {
    if matches!(error.raw_os_error(), Some(28 | 112)) {
        return format!("{action}失败：磁盘空间不足，请清理空间后重试");
    }
    match error.kind() {
        ErrorKind::NotFound => format!("{action}失败：文件或文件夹不存在，请重新选择"),
        ErrorKind::PermissionDenied => format!("{action}失败：没有访问权限，请更换位置或检查系统权限"),
        ErrorKind::WriteZero => {
            format!("{action}失败：磁盘空间不足，请清理空间后重试")
        }
        ErrorKind::InvalidData => format!("{action}失败：文件内容无效或已经损坏"),
        _ => format!("{action}失败：请确认文件可访问且磁盘状态正常"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Rect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
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
    telea_threshold: Option<u8>,
    telea_mask_radius: Option<u32>,
    telea_radius: Option<u32>,
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
    #[serde(default)]
    rotation_enabled: bool,
    #[serde(default)]
    rotation: f32,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderedPatch {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

struct PositionedImage {
    x: i32,
    y: i32,
    image: RgbaImage,
}

#[derive(Debug, Clone)]
pub(crate) struct OrientedRegion {
    pub(crate) rect: Rect,
    pub(crate) bounds: Rect,
    pivot: (f32, f32),
    sine: f32,
    cosine: f32,
}

impl OrientedRegion {
    fn new(rect: &Rect, pivot: (f32, f32), degrees: f32, width: u32, height: u32) -> Self {
        let (sine, cosine) = rotation_sin_cos(degrees);
        let raw = oriented_bounds(rect, pivot, sine, cosine);
        let left = raw.x.max(0).min(width as i32) as u32;
        let top = raw.y.max(0).min(height as i32) as u32;
        let right = (raw.x + raw.width as i32).max(0).min(width as i32) as u32;
        let bottom = (raw.y + raw.height as i32).max(0).min(height as i32) as u32;
        Self {
            rect: rect.clone(),
            bounds: Rect { x: left, y: top, width: right.saturating_sub(left), height: bottom.saturating_sub(top) },
            pivot,
            sine,
            cosine,
        }
    }

    pub(crate) fn contains(&self, x: u32, y: u32) -> bool {
        let (source_x, source_y) = self.inverse_map(x as f32 + 0.5, y as f32 + 0.5);
        source_x >= self.rect.x as f32
            && source_x < self.rect.x.saturating_add(self.rect.width) as f32
            && source_y >= self.rect.y as f32
            && source_y < self.rect.y.saturating_add(self.rect.height) as f32
    }

    fn inverse_map(&self, x: f32, y: f32) -> (f32, f32) {
        let dx = x - self.pivot.0;
        let dy = y - self.pivot.1;
        (
            self.pivot.0 + self.cosine * dx + self.sine * dy,
            self.pivot.1 - self.sine * dx + self.cosine * dy,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct PixelBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn normalized_rotation(field: &TemplateField) -> f32 {
    if !field.rotation_enabled || !field.rotation.is_finite() {
        return 0.0;
    }
    let normalized = (field.rotation + 180.0).rem_euclid(360.0) - 180.0;
    if normalized.abs() < 0.0001 { 0.0 } else { normalized }
}

fn rotation_sin_cos(degrees: f32) -> (f32, f32) {
    let (mut sine, mut cosine) = degrees.to_radians().sin_cos();
    if sine.abs() < 0.000001 { sine = 0.0; }
    if cosine.abs() < 0.000001 { cosine = 0.0; }
    (sine, cosine)
}

fn field_pivot(field: &TemplateField) -> (f32, f32) {
    (
        field.layout_rect.x as f32 + field.layout_rect.width as f32 / 2.0,
        field.layout_rect.y as f32 + field.layout_rect.height as f32 / 2.0,
    )
}

fn oriented_bounds(rect: &Rect, pivot: (f32, f32), sine: f32, cosine: f32) -> PixelBounds {
    if rect.width == 0 || rect.height == 0 {
        return PixelBounds { x: rect.x as i32, y: rect.y as i32, width: 0, height: 0 };
    }
    let left = rect.x as f32;
    let top = rect.y as f32;
    let right = rect.x.saturating_add(rect.width) as f32;
    let bottom = rect.y.saturating_add(rect.height) as f32;
    let corners = [(left, top), (right, top), (right, bottom), (left, bottom)];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (x, y) in corners {
        let dx = x - pivot.0;
        let dy = y - pivot.1;
        let rotated_x = pivot.0 + cosine * dx - sine * dy;
        let rotated_y = pivot.1 + sine * dx + cosine * dy;
        min_x = min_x.min(rotated_x); min_y = min_y.min(rotated_y);
        max_x = max_x.max(rotated_x); max_y = max_y.max(rotated_y);
    }
    let x = min_x.floor() as i32;
    let y = min_y.floor() as i32;
    let right = max_x.ceil() as i32;
    let bottom = max_y.ceil() as i32;
    PixelBounds { x, y, width: right.saturating_sub(x) as u32, height: bottom.saturating_sub(y) as u32 }
}

fn erase_region(field: &TemplateField, width: u32, height: u32) -> OrientedRegion {
    OrientedRegion::new(&field.erase_rect, field_pivot(field), normalized_rotation(field), width, height)
}

fn encode_positioned_patch(positioned: PositionedImage) -> AppResult<RenderedPatch> {
    let width = positioned.image.width();
    let height = positioned.image.height();
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(positioned.image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|_| "预览图层编码失败，请缩小处理区域后重试".to_owned())?;
    Ok(RenderedPatch { x: positioned.x, y: positioned.y, width, height, bytes: bytes.into_inner() })
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
    let raw = fs::read_to_string(path).map_err(|e| io_error_message("读取模板", &e))?;
    let doc: TemplateDocument = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "模板内容格式不正确（第 {} 行，第 {} 列），请选择由像素铸坊保存的模板",
            e.line(),
            e.column()
        )
    })?;
    if doc.schema_version != 1 {
        return Err(format!(
            "模板版本 {} 暂不受支持，当前应用支持版本 1",
            doc.schema_version
        ));
    }
    Ok(doc)
}

#[tauri::command]
fn save_template(path: String, template: TemplateDocument) -> AppResult<()> {
    let json = serde_json::to_string_pretty(&template)
        .map_err(|_| "无法整理模板数据，请检查字段设置后重试".to_owned())?;
    fs::write(path, json).map_err(|e| io_error_message("保存模板", &e))
}

#[tauri::command]
fn render_clear_patch(background_path: String, field: TemplateField) -> AppResult<RenderedPatch> {
    let pristine = open_oriented_image(&background_path)?.to_rgba8();
    let region = erase_region(&field, pristine.width(), pristine.height());
    if region.bounds.width == 0 || region.bounds.height == 0 {
        return Err("背景清除区域超出图片范围".into());
    }
    if field.clear.mode == "patch" {
        let patch = clear_patch::load(field.clear.patch_path.as_deref(), &field.name, &region)?;
        return encode_positioned_patch(render_oriented_image(
            &patch,
            &field.erase_rect,
            field_pivot(&field),
            normalized_rotation(&field),
        ));
    }
    let mut result = pristine.clone();
    clear_region(&mut result, &pristine, &field)?;
    let mut patch = result
        .view(region.bounds.x, region.bounds.y, region.bounds.width, region.bounds.height)
        .to_image();
    for y in 0..patch.height() {
        for x in 0..patch.width() {
            let global_x = region.bounds.x + x;
            let global_y = region.bounds.y + y;
            if patch.get_pixel(x, y) == pristine.get_pixel(global_x, global_y) {
                patch.get_pixel_mut(x, y)[3] = 0;
            }
        }
    }
    encode_positioned_patch(PositionedImage { x: region.bounds.x as i32, y: region.bounds.y as i32, image: patch })
}

#[tauri::command]
fn render_text_patch(field: TemplateField, value: String) -> AppResult<RenderedPatch> {
    encode_positioned_patch(render_text_layer(&field, &value)?)
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
            .map_err(|_| "PNG 预览生成失败，请检查图片尺寸后重试".to_owned())?;
    } else if matches!(output_format.to_ascii_lowercase().as_str(), "jpg" | "jpeg") {
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(canvas).to_rgb8())
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .map_err(|_| "JPG 预览生成失败，请检查图片尺寸后重试".to_owned())?;
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
    fs::create_dir_all(&output_dir).map_err(|e| io_error_message("创建输出目录", &e))?;
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
                .save(&output_path)
                .map_err(|_| {
                    format!(
                        "第 {} 张 PNG 保存失败，请确认输出目录可写且磁盘空间充足",
                        index + 1
                    )
                })?;
        } else {
            image::DynamicImage::ImageRgba8(canvas)
                .to_rgb8()
                .save(&output_path)
                .map_err(|_| {
                    format!(
                        "第 {} 张 JPG 保存失败，请确认输出目录可写且磁盘空间充足",
                        index + 1
                    )
                })?;
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
        clear_region(&mut canvas, &pristine, field)
            .map_err(|error| format!("处理字段“{}”的背景时失败：{error}", field.name))?;
    }
    for field in template.fields.iter().filter(|field| field.enabled) {
        let value = resolve_value(field, row);
        draw_field(&mut canvas, field, &value)
            .map_err(|error| format!("绘制字段“{}”时失败：{error}", field.name))?;
    }
    Ok(canvas)
}

fn read_csv(path: &str) -> AppResult<Vec<HashMap<String, String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|_| "无法打开 CSV 文件，请确认文件仍然存在且有读取权限".to_owned())?;
    let headers = reader
        .headers()
        .map_err(|_| "CSV 表头读取失败，请确认第一行是有效的 UTF-8 列名".to_owned())?
        .clone();
    reader
        .records()
        .map(|record| {
            let record = record.map_err(|e| match e.position() {
                Some(position) => format!(
                    "CSV 第 {} 行数据格式不正确，请检查列数和文件编码",
                    position.line()
                ),
                None => "CSV 数据格式不正确，请检查列数和文件编码".to_owned(),
            })?;
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
        .map_err(|e| io_error_message("打开图片", &e))?
        .with_guessed_format()
        .map_err(|_| "无法识别图片格式，请确认文件未损坏且为 PNG、JPG 或 JPEG".to_owned())?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| "图片格式无效或文件已经损坏，请重新选择图片".to_owned())?;
    let orientation = decoder
        .orientation()
        .map_err(|_| "无法读取图片方向信息，请尝试重新导出该图片".to_owned())?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| "图片解码失败，文件可能已经损坏".to_owned())?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn clear_region(
    canvas: &mut RgbaImage,
    pristine: &RgbaImage,
    field: &TemplateField,
) -> AppResult<()> {
    let region = erase_region(field, canvas.width(), canvas.height());
    if region.bounds.width == 0 || region.bounds.height == 0 {
        return Ok(());
    }
    match field.clear.mode.as_str() {
        "inpaint" => clear_inpaint::apply(
            canvas,
            pristine,
            &region,
            field.clear.inpaint_threshold.unwrap_or(14),
            field.clear.inpaint_radius.unwrap_or(2).min(6),
        ),
        "telea" => clear_telea::apply(
            canvas,
            pristine,
            &region,
            field.clear.telea_threshold.unwrap_or(14),
            field.clear.telea_mask_radius.unwrap_or(1),
            field.clear.telea_radius.unwrap_or(3),
        )?,
        "patch" => clear_patch::apply(canvas, &region, field.clear.patch_path.as_deref(), &field.name)?,
        mode => return Err(format!("未知清除模式：{mode}")),
    }
    Ok(())
}

pub(crate) fn composite_region(canvas: &mut RgbaImage, source: &RgbaImage, region: &OrientedRegion) {
    if region.bounds.width == 0 || region.bounds.height == 0 {
        return;
    }
    for y in region.bounds.y..region.bounds.y + region.bounds.height {
        for x in region.bounds.x..region.bounds.x + region.bounds.width {
            let (source_x, source_y) = region.inverse_map(x as f32 + 0.5, y as f32 + 0.5);
            let sample_x = source_x - region.rect.x as f32 - 0.5;
            let sample_y = source_y - region.rect.y as f32 - 0.5;
            let pixel = sample_bilinear(source, sample_x, sample_y);
            if pixel[3] > 0 {
                blend_over(canvas.get_pixel_mut(x, y), pixel);
            }
        }
    }
}

fn render_oriented_image(source: &RgbaImage, rect: &Rect, pivot: (f32, f32), degrees: f32) -> PositionedImage {
    if rect.width == 0 || rect.height == 0 {
        return PositionedImage { x: rect.x as i32, y: rect.y as i32, image: RgbaImage::new(1, 1) };
    }
    let (sine, cosine) = rotation_sin_cos(degrees);
    let bounds = oriented_bounds(rect, pivot, sine, cosine);
    let mut output = RgbaImage::new(bounds.width.max(1), bounds.height.max(1));
    for y in 0..bounds.height {
        for x in 0..bounds.width {
            let global_x = bounds.x as f32 + x as f32 + 0.5;
            let global_y = bounds.y as f32 + y as f32 + 0.5;
            let dx = global_x - pivot.0;
            let dy = global_y - pivot.1;
            let source_x = pivot.0 + cosine * dx + sine * dy;
            let source_y = pivot.1 - sine * dx + cosine * dy;
            let sample = sample_bilinear(
                source,
                source_x - rect.x as f32 - 0.5,
                source_y - rect.y as f32 - 0.5,
            );
            output.put_pixel(x, y, sample);
        }
    }
    PositionedImage { x: bounds.x, y: bounds.y, image: output }
}

fn sample_bilinear(image: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let weights = [
        ((x0, y0), (1.0 - tx) * (1.0 - ty)),
        ((x0 + 1, y0), tx * (1.0 - ty)),
        ((x0, y0 + 1), (1.0 - tx) * ty),
        ((x0 + 1, y0 + 1), tx * ty),
    ];
    let mut alpha = 0.0f32;
    let mut premultiplied = [0.0f32; 3];
    for ((sample_x, sample_y), weight) in weights {
        if sample_x < 0 || sample_y < 0 || sample_x >= image.width() as i32 || sample_y >= image.height() as i32 {
            continue;
        }
        let pixel = image.get_pixel(sample_x as u32, sample_y as u32);
        let pixel_alpha = pixel[3] as f32 / 255.0;
        alpha += pixel_alpha * weight;
        for channel in 0..3 {
            premultiplied[channel] += pixel[channel] as f32 * pixel_alpha * weight;
        }
    }
    if alpha <= f32::EPSILON {
        return Rgba([0, 0, 0, 0]);
    }
    Rgba([
        (premultiplied[0] / alpha).round().clamp(0.0, 255.0) as u8,
        (premultiplied[1] / alpha).round().clamp(0.0, 255.0) as u8,
        (premultiplied[2] / alpha).round().clamp(0.0, 255.0) as u8,
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    ])
}

fn blend_over(destination: &mut Rgba<u8>, source: Rgba<u8>) {
    let source_alpha = source[3] as f32 / 255.0;
    if source_alpha <= 0.0 {
        return;
    }
    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= f32::EPSILON {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }
    for channel in 0..3 {
        let value = (source[channel] as f32 * source_alpha
            + destination[channel] as f32 * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
        destination[channel] = value.round().clamp(0.0, 255.0) as u8;
    }
    destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn overlay_positioned(canvas: &mut RgbaImage, layer: &PositionedImage) {
    for y in 0..layer.image.height() {
        let destination_y = layer.y + y as i32;
        if destination_y < 0 || destination_y >= canvas.height() as i32 {
            continue;
        }
        for x in 0..layer.image.width() {
            let destination_x = layer.x + x as i32;
            if destination_x < 0 || destination_x >= canvas.width() as i32 {
                continue;
            }
            let source = *layer.image.get_pixel(x, y);
            if source[3] > 0 {
                blend_over(canvas.get_pixel_mut(destination_x as u32, destination_y as u32), source);
            }
        }
    }
}

fn draw_field(canvas: &mut RgbaImage, field: &TemplateField, value: &str) -> AppResult<()> {
    if value.is_empty() {
        return Ok(());
    }
    let layer = render_text_layer(field, value)?;
    overlay_positioned(canvas, &layer);
    Ok(())
}

fn render_text_layer(field: &TemplateField, value: &str) -> AppResult<PositionedImage> {
    let width = field.layout_rect.width.max(1);
    let height = field.layout_rect.height.max(1);
    let mut local = RgbaImage::new(width, height);
    if !value.is_empty() {
        draw_text_in_rect(&mut local, &field.text, &Rect { x: 0, y: 0, width, height }, value)?;
    }
    Ok(render_oriented_image(
        &local,
        &field.layout_rect,
        field_pivot(field),
        normalized_rotation(field),
    ))
}

fn draw_text_in_rect(canvas: &mut RgbaImage, style: &TextStyle, r: &Rect, value: &str) -> AppResult<()> {
    let font = load_font(style)?;
    let scale = PxScale::from(style.font_size.max(1.0));
    let (tw, _) = text_size(scale, &font, value);
    let (glyph_min_y, glyph_max_y) = glyph_vertical_bounds(&font, scale, value);
    let pad = style.padding;
    let inner_w = r.width.saturating_sub(pad * 2);
    let x = match style.horizontal_align.as_str() {
        "right" => r.x + pad + inner_w.saturating_sub(tw),
        "center" => r.x + pad + inner_w.saturating_sub(tw) / 2,
        _ => r.x + pad,
    };
    let line_height = style.font_size.max(1.0) * style.line_height.max(0.1);
    let line_top = line_box_top(
        r.y as f32,
        r.height as f32,
        pad as f32,
        line_height,
        &style.vertical_align,
    );
    // imageproc positions glyphs relative to the font baseline and adds px_bounds.min.y
    // during drawing. Offset by the actual glyph-box center so it matches Konva's
    // middle-baseline line box instead of being shifted down by the ascent.
    let target_center = line_top + line_height / 2.0;
    let glyph_center = (glyph_min_y + glyph_max_y) / 2.0;
    let y = (target_center - glyph_center).round() as i32;
    draw_text_mut(
        canvas,
        parse_color(&style.color)?,
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
        if let Ok(bytes) = fs::read(path)
            && let Ok(font) = FontArc::try_from_vec(bytes)
        {
            return Ok(font);
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
            render_clear_patch,
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

    #[test]
    fn oriented_bounds_rotate_around_the_layout_center() {
        let rect = Rect { x: 30, y: 30, width: 40, height: 20 };
        let (sine, cosine) = rotation_sin_cos(90.0);
        let bounds = oriented_bounds(&rect, (50.0, 40.0), sine, cosine);
        assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height), (40, 20, 20, 40));
    }

    #[test]
    fn oriented_region_rejects_pixels_in_its_axis_aligned_corners() {
        let rect = Rect { x: 30, y: 35, width: 40, height: 10 };
        let region = OrientedRegion::new(&rect, (50.0, 40.0), 45.0, 100, 80);
        assert!(region.contains(50, 40));
        assert!(!region.contains(region.bounds.x, region.bounds.y));
    }

    #[test]
    fn zero_degree_layer_keeps_source_pixels_and_position() {
        let mut source = RgbaImage::new(2, 1);
        source.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        source.put_pixel(1, 0, Rgba([40, 50, 60, 128]));
        let rect = Rect { x: 12, y: 8, width: 2, height: 1 };
        let layer = render_oriented_image(&source, &rect, (13.0, 8.5), 0.0);
        assert_eq!((layer.x, layer.y), (12, 8));
        assert_eq!(layer.image, source);
    }
}
