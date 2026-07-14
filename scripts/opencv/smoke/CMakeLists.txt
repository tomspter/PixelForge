cmake_minimum_required(VERSION 3.20)
project(pixelforge_opencv_smoke LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(OpenCV_STATIC ON)

find_package(OpenCV CONFIG REQUIRED COMPONENTS core imgproc photo)

add_executable(opencv-smoke main.cpp)
target_link_libraries(opencv-smoke PRIVATE ${OpenCV_LIBS})

enable_testing()
add_test(NAME opencv-inpaint-smoke COMMAND opencv-smoke)
