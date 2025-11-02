set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(TARGET whisper-worker)
add_executable(${TARGET} worker.cpp)

include(DefaultTargetOptions)

target_link_libraries(${TARGET} PRIVATE common whisper ${CMAKE_THREAD_LIBS_INIT})

# Link OpenSSL for WebSocket handshake
find_package(OpenSSL REQUIRED)
target_link_libraries(${TARGET} PRIVATE OpenSSL::SSL OpenSSL::Crypto)

if (WIN32)
    target_link_libraries(${TARGET} PRIVATE ws2_32)
endif()

install(TARGETS ${TARGET} RUNTIME)

