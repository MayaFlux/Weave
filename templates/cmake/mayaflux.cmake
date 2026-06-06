# ============================================================================
# MayaFlux Discovery
# ============================================================================

set(MAYAFLUX_SEARCH_PATHS "")

if(DEFINED ENV{MAYAFLUX_ROOT})
    list(APPEND MAYAFLUX_SEARCH_PATHS "$ENV{MAYAFLUX_ROOT}/lib/cmake/MayaFlux")
    list(APPEND MAYAFLUX_SEARCH_PATHS "$ENV{MAYAFLUX_ROOT}/lib64/cmake/MayaFlux")
elseif(PLATFORM_MACOS)
    list(APPEND MAYAFLUX_SEARCH_PATHS "/opt/homebrew/lib/cmake/MayaFlux")
    list(APPEND MAYAFLUX_SEARCH_PATHS "/usr/local/lib/cmake/MayaFlux")
elseif(PLATFORM_LINUX)
    list(APPEND MAYAFLUX_SEARCH_PATHS "/usr/lib/cmake/MayaFlux")
    list(APPEND MAYAFLUX_SEARCH_PATHS "/usr/lib64/cmake/MayaFlux")
    list(APPEND MAYAFLUX_SEARCH_PATHS "/usr/local/lib/cmake/MayaFlux")
elseif(PLATFORM_WINDOWS)
    list(APPEND MAYAFLUX_SEARCH_PATHS "C:/MayaFlux/lib/cmake/MayaFlux")
endif()

find_package(MayaFlux REQUIRED
    PATHS ${MAYAFLUX_SEARCH_PATHS}
    NO_DEFAULT_PATH
)

if(NOT MayaFlux_FOUND)
    message(FATAL_ERROR
        "MayaFlux not found!\n"
        "Searched in:\n  ${MAYAFLUX_SEARCH_PATHS}\n"
        "\nPlease ensure MayaFlux is installed or set MAYAFLUX_ROOT environment variable.\n"
        "Installation instructions: https://github.com/MayaFlux/MayaFlux"
    )
endif()

if(NOT EXISTS "${MayaFlux_PCH_FILE}")
    message(FATAL_ERROR
        "MayaFlux runtime PCH not found at ${MayaFlux_PCH_FILE}\n"
        "Ensure MayaFlux is correctly installed."
    )
endif()

target_precompile_headers(${PROJECT_NAME} PUBLIC "${MayaFlux_PCH_FILE}")

message(STATUS "Found MayaFlux ${MayaFlux_VERSION}: ${MayaFlux_DIR}")

# ============================================================================
# MayaFlux Linking
# ============================================================================
target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxLib)

# Optional: Lila (Live Coding JIT)
@LILA_LINK_BLOCK@

# ============================================================================
# Platform-Specific Configuration
# ============================================================================

if(PLATFORM_MACOS)
    if(DEFINED MayaFlux_RPATH_HINTS)
        list(JOIN MayaFlux_RPATH_HINTS ";" RPATH_LIST)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            BUILD_RPATH "${RPATH_LIST}"
            INSTALL_RPATH "${RPATH_LIST}"
            BUILD_WITH_INSTALL_RPATH TRUE
            MACOSX_RPATH ON
        )
    else()
        set_target_properties(${PROJECT_NAME} PROPERTIES
            BUILD_RPATH "@loader_path/../lib;${MayaFlux_LIB_DIR}"
            INSTALL_RPATH "@loader_path/../lib;${MayaFlux_LIB_DIR}"
            BUILD_WITH_INSTALL_RPATH TRUE
            MACOSX_RPATH ON
        )
    endif()

elseif(PLATFORM_LINUX)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        INSTALL_RPATH "$ORIGIN/../lib:$ORIGIN:${MayaFlux_LIB_DIR}"
        BUILD_WITH_INSTALL_RPATH TRUE
    )

    find_package(Threads REQUIRED)
    target_link_libraries(${PROJECT_NAME} PRIVATE Threads::Threads)

elseif(PLATFORM_WINDOWS)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        MSVC_RUNTIME_LIBRARY "MultiThreadedDLL"
        VS_DEBUGGER_ENVIRONMENT "PATH=$<TARGET_FILE_DIR:MayaFlux::MayaFluxLib>;@LILA_DEBUGGER_PATH@$ENV{PATH}"
    )

    if(DEFINED MayaFlux_RUNTIME_LIBRARIES)
        foreach(_DLL ${MayaFlux_RUNTIME_LIBRARIES})
            if(EXISTS "${_DLL}")
                add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "${_DLL}"
                        $<TARGET_FILE_DIR:${PROJECT_NAME}>
                    COMMENT "Copying ${_DLL}"
                )
            endif()
        endforeach()
    else()
        if(DEFINED ENV{MAYAFLUX_ROOT})
            if(EXISTS "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxLib.dll")
                add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxLib.dll"
                        $<TARGET_FILE_DIR:${PROJECT_NAME}>
                )
            endif()

            @LILA_DLL_COPY@
        endif()
    endif()
    
    # Windows subsystem configuration
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        # Release builds hide console window (optional - comment out if you want console)
        # set_target_properties(${PROJECT_NAME} PROPERTIES WIN32_EXECUTABLE TRUE)
    endif()
endif()



# ============================================================================
# Shaders
# ============================================================================
include(cmake/shaders)

target_compile_definitions(${PROJECT_NAME} PRIVATE
    MAYAFLUX_PROJECT_SHADER_DIR="${CMAKE_SOURCE_DIR}/data/shaders"
)

if(TARGET compile_project_shaders)
    add_dependencies(${PROJECT_NAME} compile_project_shaders)
endif()

# ============================================================================
# Community Modules
# ============================================================================
include(cmake/build_community)

# ============================================================================
# Configuration Options
# ============================================================================

option(MAYAFLUX_CONFIG_OVERRIDE
       "JSON config file takes precedence over settings()" OFF)
