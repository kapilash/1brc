# 1brc
A C++ (Boost) implementation for [One Billion Row Challenge](https://www.morling.dev/blog/one-billion-row-challenge/) 

# Compiling

The usual CMake style build

### windows (with vcpkg)
```
cd <folder-containing-source>
mkdir build
cd build
cmake .. -DCMAKE_TOOLCHAIN_FILE=c:\vcpkg\scripts\buildsystems\vcpkg.cmake -G "Visual Studio 17 2022" -A x64 -DCMAKE_BUILD_TYPE=Release
msbuild onebrc.sln /p:Configuration=Release
```
or 
### via ninja
```
cmake .. -G Ninja -DCMAKE_BUILD_TYPE=Release
ninja
```

# Influences

Gratefully acknowledging the following influences

1. Keeping the temperatures as ints till the time of actually printing out the result is copied from [buybackoff's super impressive dotnet implementation](https://github.com/buybackoff)
2. [Gopal](https://github.com/kasturgo) convinced me to do a preprocessing to create self-contained batches.
    
# Numbers

Following are the timings on my machine

 | Implementation   |   Time  |             
 | ---------------- | ------- |
 |[java baseline from gunnarmorling](https://github.com/gunnarmorling/1brc) | ~2 min |
 | this implementation                                               | 2.8s |
--------------------------------------------------------------------------
