# Minimal OpenCV build

The `opencv-static.yml` workflow builds a pinned OpenCV revision with only
`core`, `imgproc`, and `photo`. It does not modify or publish the PixelForge
application.

Each matrix job:

1. builds a position-independent static OpenCV installation;
2. links and runs a small `cv::inpaint` smoke test;
3. packages headers, CMake metadata, static libraries, the OpenCV license, and
   a machine-readable manifest;
4. writes uncompressed, compressed, and per-library sizes to the GitHub Actions
   job summary;
5. uploads a ZIP, SHA-256 file, and Markdown size report for 30 days.

The four artifacts target Windows x64, Linux x64, macOS ARM64, and macOS Intel.
The two macOS artifacts are intentionally separate so a later Tauri universal
build can link each Rust target against the matching OpenCV architecture before
combining the final application binaries.

Unix builds use the single-config `MinSizeRel` mode. OpenCV's Visual Studio
projects expose only `Debug` and `Release`, so Windows uses the optimized
`Release` configuration. `WITH_ADE` is explicitly disabled on every platform:
G-API is outside this minimal module set, and leaving its initialization hook
enabled would export an unbuilt `ade` archive into `OpenCVModules.cmake`.

To update OpenCV, change both `OPENCV_VERSION` and the full peeled commit SHA in
`.github/workflows/opencv-static.yml`.
