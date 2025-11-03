# Prevent in-source builds

## Problem

Running `cmake .` from the project root directory creates build artifacts (CMakeCache.txt, CMakeFiles/, Makefiles, etc.) directly in the source tree, polluting the repository with untracked files.

## Solution

Added a check in `CMakeLists.txt` that prevents in-source builds by detecting when source and binary directories are the same. CMake will now fail with a clear error message instructing users to use a separate build directory:

```bash
mkdir build
cd build
cmake ..
make
```

This follows CMake best practices and keeps the source tree clean.

