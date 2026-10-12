cmake_minimum_required(VERSION 3.18)
project(geoprims_s2_smoke LANGUAGES CXX)

find_package(s2 CONFIG REQUIRED)

add_executable(s2-smoke main.cc)
target_compile_features(s2-smoke PRIVATE cxx_std_17)
target_link_libraries(s2-smoke PRIVATE s2::s2)
