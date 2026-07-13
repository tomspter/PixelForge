# 像素铸坊：架构与实现说明

## 1. 边界与原则

- Vue 只负责模板编辑、即时预览、交互状态；Rust 是最终 PNG/JPG 输出的唯一可信实现。
- 模板坐标永远是原图像素。Konva Stage 只通过整体 `scale` 显示缩放，字段模型不保存屏幕坐标。
- `eraseRect` 与 `layoutRect` 独立建模。前者确保完全盖住旧内容，后者只负责新文字排版。
- 背景、遮罩、文字、交互四层隔离，导出不包含 Interaction Layer。
- JSON 带 `schemaVersion`，后续增加图片字段、二维码或多行排版时可迁移而不破坏旧模板。

## 2. 模块划分

| 模块 | 职责 | 采用组件 |
|---|---|---|
| EditorCanvas | 分层画布、画框、选择、拖拽、缩放 | vue-konva / Konva Transformer |
| Pinia template store | 文档、选择态、工具态、保存/生成命令编排 | Pinia |
| FieldList / PropertyPanel | 字段管理与配置 | Vue + daisyUI |
| BatchDrawer | CSV、输出目录、生成反馈 | Tauri dialog plugin |
| Tauri commands | 图片检查、JSON I/O、批量任务入口 | Tauri 2 command |
| Rust renderer | 遮罩合成、字段求值、文字绘制、PNG/JPG 编码 | image / imageproc / ab_glyph / csv |

## 3. 数据流

1. 导入 PNG、JPG 或 JPEG，Rust 解码并读取真实尺寸；前端建立空模板。
2. 用户在缩放后的 Stage 上画框，指针坐标除以 Stage scale 后写入模型。
3. 预览按照 Background → Mask → Text → Interaction 顺序响应式重绘。
4. 保存时将完整 `TemplateDocument` 序列化为可读 JSON；背景仅保存路径，不复制大文件。
5. 批量生成时 Rust 逐行读取 CSV；每一行重新解码一份干净背景，逐字段清除再绘字，最后按用户选择编码 PNG 或 JPG。

## 4. 清除策略

| 模式 | 适用场景 | 实现 |
|---|---|---|
| `inpaint` | 扫描件中的深色文字，默认推荐 | 局部背景差分生成文字掩码，仅对笔画像素做邻域修补 |
| `patch` | 固定纹理模板，推荐 | 补丁缩放到 `eraseRect` 后覆盖 |

所有策略都从 pristine 背景取样，避免前一个字段的修改污染后续字段。

智能抹除的 `inpaintThreshold` 越低，进入掩码的深色像素越多；`inpaintRadius` 用于覆盖文字边缘的抗锯齿。该模式无法在没有语义信息的情况下区分文字与穿过画框的表格线，因此画框应尽量只包住原文字。

## 5. 模板 JSON 示例

```json
{
  "schemaVersion": 1,
  "id": "uuid",
  "name": "证书模板",
  "background": { "path": "/templates/certificate.png", "width": 2480, "height": 3508 },
  "fields": [{
    "id": "uuid", "name": "姓名", "kind": "csv", "value": "",
    "csvColumn": "name",
    "eraseRect": { "x": 720, "y": 1320, "width": 1040, "height": 180 },
    "layoutRect": { "x": 760, "y": 1340, "width": 960, "height": 140 },
    "text": { "fontFamily": "宋体", "fontSize": 72, "color": "#181818", "horizontalAlign": "center", "verticalAlign": "middle", "lineHeight": 1.2, "letterSpacing": 0, "padding": 4 },
    "clear": { "mode": "patch", "color": "#ffffff", "patchPath": "/templates/patches/name.png" },
    "enabled": true
  }],
  "createdAt": "2026-07-12T10:00:00Z", "updatedAt": "2026-07-12T10:00:00Z"
}
```

## 6. 扩展建议

- 第二阶段将生成命令改为 Tauri Channel，按行推送进度并支持取消；核心 renderer 保持不变。
- 通过模板资源目录打包背景、补丁和字体，并在 JSON 中保存相对路径，提高模板可迁移性。
- 多行/溢出策略可接入 `cosmic-text`；当前 `imageproc + ab_glyph` 更轻，适合第一阶段单行固定模板。
- 大批量时使用有限工作线程并行生成，但需按内存预算限制并发，避免多份大 PNG 同时驻留。
