set(TARGET whisper-system-info)
add_executable(${TARGET} system-info.cpp)
target_link_libraries(${TARGET} PRIVATE whisper ${CMAKE_THREAD_LIBS_INIT})
set_target_properties(${TARGET} PROPERTIES OUTPUT_NAME whisper-system)
install(TARGETS ${TARGET} RUNTIME)

