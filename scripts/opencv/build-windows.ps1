param(
  [Parameter(Mandatory = $true)][string]$SourceDir,
  [Parameter(Mandatory = $true)][string]$BuildDir,
  [Parameter(Mandatory = $true)][string]$InstallDir,
  [Parameter(Mandatory = $true)][string]$SmokeBuildDir
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $BuildDir, $InstallDir, $SmokeBuildDir

$ConfigureArgs = @(
  "-S", $SourceDir,
  "-B", $BuildDir,
  "-A", "x64",
  "-DCMAKE_INSTALL_PREFIX=$InstallDir",
  "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
  "-DBUILD_LIST=core,imgproc,photo",
  "-DBUILD_SHARED_LIBS=OFF",
  "-DBUILD_WITH_STATIC_CRT=OFF",
  "-DBUILD_opencv_world=OFF",
  "-DBUILD_TESTS=OFF",
  "-DBUILD_PERF_TESTS=OFF",
  "-DBUILD_EXAMPLES=OFF",
  "-DBUILD_opencv_apps=OFF",
  "-DBUILD_JAVA=OFF",
  "-DBUILD_opencv_python2=OFF",
  "-DBUILD_opencv_python3=OFF",
  "-DBUILD_DOCS=OFF",
  "-DBUILD_PACKAGE=OFF",
  "-DOPENCV_GENERATE_PKGCONFIG=OFF",
  "-DOPENCV_ENABLE_NONFREE=OFF",
  # G-API's init hook otherwise exports an unbuilt EXCLUDE_FROM_ALL ADE target.
  "-DWITH_ADE=OFF",
  "-DWITH_1394=OFF",
  "-DWITH_CUDA=OFF",
  "-DWITH_EIGEN=OFF",
  "-DWITH_FFMPEG=OFF",
  "-DWITH_GSTREAMER=OFF",
  "-DWITH_HALIDE=OFF",
  "-DWITH_IPP=OFF",
  "-DWITH_ITT=OFF",
  "-DWITH_JASPER=OFF",
  "-DWITH_JPEG=OFF",
  "-DWITH_KLEIDICV=OFF",
  "-DWITH_LAPACK=OFF",
  "-DWITH_OPENCL=OFF",
  "-DWITH_OPENMP=OFF",
  "-DWITH_OPENEXR=OFF",
  "-DWITH_PNG=OFF",
  "-DWITH_PROTOBUF=OFF",
  "-DWITH_QT=OFF",
  "-DWITH_TBB=OFF",
  "-DWITH_TIFF=OFF",
  "-DWITH_VULKAN=OFF",
  "-DWITH_WEBP=OFF",
  "-DWITH_ZLIB_NG=OFF"
)

cmake @ConfigureArgs
# OpenCV intentionally limits Visual Studio projects to Debug and Release.
# Requesting MinSizeRel produces MSB8013 because that project configuration
# does not exist, so use the optimized Release configuration consistently.
cmake --build $BuildDir --config Release --parallel $env:NUMBER_OF_PROCESSORS
cmake --install $BuildDir --config Release

$OpenCvConfig = Get-ChildItem -Path $InstallDir -Filter OpenCVConfig.cmake -Recurse | Select-Object -First 1
if (-not $OpenCvConfig) {
  throw "OpenCVConfig.cmake was not installed"
}

cmake `
  -S (Join-Path $RepoRoot "scripts/opencv/smoke") `
  -B $SmokeBuildDir `
  -A x64 `
  "-DOpenCV_DIR=$($OpenCvConfig.Directory.FullName)" `
  -DOpenCV_STATIC=ON
cmake --build $SmokeBuildDir --config Release --parallel $env:NUMBER_OF_PROCESSORS
ctest --test-dir $SmokeBuildDir -C Release --output-on-failure
