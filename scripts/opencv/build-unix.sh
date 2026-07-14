#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <source-dir> <build-dir> <install-dir> <smoke-build-dir>" >&2
  exit 2
fi

source_dir="$1"
build_dir="$2"
install_dir="$3"
smoke_build_dir="$4"
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"

rm -rf "$build_dir" "$install_dir" "$smoke_build_dir"

cmake_args=(
  -S "$source_dir"
  -B "$build_dir"
  -G Ninja
  -DCMAKE_BUILD_TYPE=MinSizeRel
  -DCMAKE_INSTALL_PREFIX="$install_dir"
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON
  -DBUILD_LIST=core,imgproc,photo
  -DBUILD_SHARED_LIBS=OFF
  -DBUILD_opencv_world=OFF
  -DBUILD_TESTS=OFF
  -DBUILD_PERF_TESTS=OFF
  -DBUILD_EXAMPLES=OFF
  -DBUILD_opencv_apps=OFF
  -DBUILD_JAVA=OFF
  -DBUILD_opencv_python2=OFF
  -DBUILD_opencv_python3=OFF
  -DBUILD_DOCS=OFF
  -DBUILD_PACKAGE=OFF
  -DOPENCV_GENERATE_PKGCONFIG=OFF
  -DOPENCV_ENABLE_NONFREE=OFF
  -DENABLE_PIC=ON
  # G-API's init hook creates and exports the optional ADE target before
  # BUILD_LIST disables G-API. With ADE left enabled, the generated package
  # references an archive that an EXCLUDE_FROM_ALL build never creates.
  -DWITH_ADE=OFF
  -DWITH_1394=OFF
  -DWITH_CUDA=OFF
  -DWITH_EIGEN=OFF
  -DWITH_FFMPEG=OFF
  -DWITH_GSTREAMER=OFF
  -DWITH_GTK=OFF
  -DWITH_HALIDE=OFF
  -DWITH_IPP=OFF
  -DWITH_ITT=OFF
  -DWITH_JASPER=OFF
  -DWITH_JPEG=OFF
  -DWITH_KLEIDICV=OFF
  -DWITH_LAPACK=OFF
  -DWITH_OPENCL=OFF
  -DWITH_OPENMP=OFF
  -DWITH_OPENEXR=OFF
  -DWITH_PNG=OFF
  -DWITH_PROTOBUF=OFF
  -DWITH_QT=OFF
  -DWITH_TBB=OFF
  -DWITH_TIFF=OFF
  -DWITH_V4L=OFF
  -DWITH_VULKAN=OFF
  -DWITH_WEBP=OFF
  -DWITH_ZLIB_NG=OFF
)

if [[ "$(uname -s)" == "Darwin" ]]; then
  if [[ -z "${OPENCV_CMAKE_ARCH:-}" ]]; then
    echo "OPENCV_CMAKE_ARCH must be set for macOS builds" >&2
    exit 2
  fi
  cmake_args+=(
    -DCMAKE_OSX_ARCHITECTURES="$OPENCV_CMAKE_ARCH"
    -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0
  )
fi

cmake "${cmake_args[@]}"
cmake --build "$build_dir" --parallel
cmake --install "$build_dir"

opencv_config="$(find "$install_dir" -name OpenCVConfig.cmake -print -quit)"
if [[ -z "$opencv_config" ]]; then
  echo "OpenCVConfig.cmake was not installed" >&2
  exit 1
fi
opencv_dir="$(dirname "$opencv_config")"

cmake \
  -S "$repo_root/scripts/opencv/smoke" \
  -B "$smoke_build_dir" \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=MinSizeRel \
  -DOpenCV_DIR="$opencv_dir" \
  -DOpenCV_STATIC=ON
cmake --build "$smoke_build_dir" --parallel
ctest --test-dir "$smoke_build_dir" --output-on-failure
