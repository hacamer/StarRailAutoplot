# 本地 overlay triplet：内容 = vcpkg(94a50e8) 社区版 x64-windows-static-release + 环境透传
# CMAKE_POLICY_VERSION_MINIMUM 需要进到 vcpkg 子进程环境（vcpkg 会清洗环境），
# 否则 CMake 4.x 下老端口（bzip2 cmake_minimum_required 3.0）configure 直接报错。
set(VCPKG_TARGET_ARCHITECTURE x64)
set(VCPKG_CRT_LINKAGE static)
set(VCPKG_LIBRARY_LINKAGE static)

set(VCPKG_BUILD_TYPE release)

set(VCPKG_ENV_PASSTHROUGH CMAKE_POLICY_VERSION_MINIMUM)
