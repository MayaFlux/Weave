set(PROJECT_SHADER_DIR "${CMAKE_SOURCE_DIR}/data/shaders")

if(NOT EXISTS "${PROJECT_SHADER_DIR}")
    return()
endif()

file(GLOB_RECURSE PROJECT_SHADER_FILES
    "${PROJECT_SHADER_DIR}/*.vert"
    "${PROJECT_SHADER_DIR}/*.frag"
    "${PROJECT_SHADER_DIR}/*.comp"
    "${PROJECT_SHADER_DIR}/*.geom"
    "${PROJECT_SHADER_DIR}/*.tesc"
    "${PROJECT_SHADER_DIR}/*.tese"
    "${PROJECT_SHADER_DIR}/*.mesh"
    "${PROJECT_SHADER_DIR}/*.task"
    "${PROJECT_SHADER_DIR}/*.rgen"
    "${PROJECT_SHADER_DIR}/*.rchit"
    "${PROJECT_SHADER_DIR}/*.rmiss"
)

if(NOT PROJECT_SHADER_FILES)
    return()
endif()

set(PROJECT_SHADER_OUTPUTS)

foreach(SHADER_FILE ${PROJECT_SHADER_FILES})
    set(OUTPUT_FILE "${SHADER_FILE}.spv")

    add_custom_command(
        OUTPUT  "${OUTPUT_FILE}"
        COMMAND glslc "${SHADER_FILE}" -o "${OUTPUT_FILE}"
        DEPENDS "${SHADER_FILE}"
        COMMENT "Compiling shader: ${SHADER_FILE}"
        VERBATIM
    )

    list(APPEND PROJECT_SHADER_OUTPUTS "${OUTPUT_FILE}")
endforeach()

add_custom_target(compile_project_shaders DEPENDS ${PROJECT_SHADER_OUTPUTS})

message(STATUS "Project shaders: ${PROJECT_SHADER_DIR}")
