# Spaghetti Engine

A 3D game engine built in Rust with OpenGL.

## Windows Setup

### Prerequisites

1. **Install MSVC Toolchain**
   - Download and install [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - Select "Desktop development with C++" workload

2. **Install CMake**
   - Download and install [CMake 4.2.0](https://cmake.org/download/) from the official website

3. **Install Ninja**
   ```powershell
   choco install ninja -y
   ```

4. **Dependencies**
   - Libs are manually provided for the time being
   - Ensure the `libs/` folder is present with required libraries

### Running

**VS Code (recommended):**
- The run command auto-populates via Terminal Keeper extension

**Manual:**
```cmd
poop.bat
```

