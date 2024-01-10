cmake_minimum_required (VERSION 3.20)

project(onebrc VERSION 0.1 LANGUAGES CXX)

set(Boost_USE_MULTITHREADED ON)
set(Boost_USE_STATIC_LIBS ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

FIND_PACKAGE(Boost REQUIRED COMPONENTS system filesystem date_time)
message(STATUS "Boost_LIBRARIES = ${Boost_LIBRARIES}")

if (CMAKE_CXX_COMPILER_ID MATCHES "Clang")
 add_compile_options(-std=c++14 -stdlib=libc++ -O2)
 set(CMAKE_CXX_FLAGS "-Wall -Wextra")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
 add_compile_options(-std=c++14 -O3)
 set(CMAKE_CXX_FLAGS "-Wall -Wextra")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "MSVC")
 add_compile_options( -openmp:experimental /FAc)
endif()

add_executable(onebrc OneBRC.cpp) 

target_link_libraries(onebrc PRIVATE ${Boost_LIBRARIES})
target_include_directories(onebrc PRIVATE ${Boost_INCLUDE_DIRS})
