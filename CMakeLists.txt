cmake_minimum_required(VERSION 3.28.1)

project(alarm-server)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall")
set(CMAKE_CXX_STANDARD 23)


add_executable(alarm-server main.cpp)