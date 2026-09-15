# Image Processor with Plugins

CLI-приложение модуля 4: загружает изображение, применяет динамический плагин и сохраняет результат в PNG.

## Структура

Cargo workspace из четырёх крейтов:

- `image_processor` — бинарный хост (clap + image + libloading)
- `plugin_interface` — общая FFI-сигнатура `ProcessImageFn`
- `mirror_plugin` — cdylib `mirror` (горизонтальное/вертикальное отражение)
- `blur_plugin` — cdylib `blur` (box blur)

## Сборка

```bash
cargo build --workspace
```

Плагины появятся в `target/debug` как `mirror.dll` / `blur.dll` (Windows), `libmirror.so` / `libblur.so` (Linux) или `libmirror.dylib` / `libblur.dylib` (macOS).

## Запуск

```bash
# Зеркальный разворот
cargo run -p image_processor -- \
  --input input.png \
  --output out_mirror.png \
  --plugin mirror \
  --params params/mirror.txt

# Размытие
cargo run -p image_processor -- \
  --input input.png \
  --output out_blur.png \
  --plugin blur \
  --params params/blur.txt
```

Опциональный флаг `--plugin-path` (по умолчанию `target/debug`) указывает каталог с библиотеками плагинов. В `--plugin` передаётся только голое имя (`mirror`, `blur`).

## Параметры

Формат файла `--params`: пары `key=value` через запятую и/или перевод строки.

- **mirror**: `horizontal`, `vertical` (bool, по умолчанию `false`)
- **blur**: `radius`, `iterations` (u32, по умолчанию `1`)

## Тесты

```bash
cargo test --workspace
```
