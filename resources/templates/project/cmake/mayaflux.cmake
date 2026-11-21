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
    target_link_options(${PROJECT_NAME} PRIVATE -rdynamic)

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

        endif()
    endif()

    # WEAVE: Lila Windows runtime integration
    @LILA_DLL_COPY@

    # Windows subsystem configuration
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        # Release builds hide console window (optional - comment out if you want console)
        # set_target_properties(${PROJECT_NAME} PROPERTIES WIN32_EXECUTABLE TRUE)
    endif()
endif()



# ============================================================================
# Reflection: MF_DESCRIBE and MF_DESCRIBE_FUNCTION sidecars
# ============================================================================

option(MAYAFLUX_REFLECT_BUILD "Generate MF_DESCRIBE and MF_DESCRIBE_FUNCTION sidecars before building" ON)

get_filename_component(MAYAFLUX_PREFIX "${MayaFlux_DIR}/../../.." ABSOLUTE)

set(MAYAFLUX_DESCRIBED_HEADERS "")
set(MAYAFLUX_DESCRIBED_FUNCTION_HEADERS "")
foreach(_header IN LISTS HEADERS)
    if(NOT _header MATCHES "\\.(h|hh|hpp|hxx)$" OR _header MATCHES "\\.(describe|signatures)\\.[^.]+$")
        continue()
    endif()

    file(STRINGS "${_header}" _request REGEX "^[ \t;}]*MF_DESCRIBE[ \t]*\\(" LIMIT_COUNT 1)
    if(_request)
        list(APPEND MAYAFLUX_DESCRIBED_HEADERS "${_header}")
    endif()

    file(STRINGS "${_header}" _request REGEX "^[ \t;}]*MF_DESCRIBE_FUNCTION[ \t]*\\(" LIMIT_COUNT 1)
    if(_request)
        list(APPEND MAYAFLUX_DESCRIBED_FUNCTION_HEADERS "${_header}")
    endif()
endforeach()

if(MAYAFLUX_REFLECT_BUILD AND (MAYAFLUX_DESCRIBED_HEADERS OR MAYAFLUX_DESCRIBED_FUNCTION_HEADERS) AND USER_SOURCES)
    if(NOT CMAKE_GENERATOR MATCHES "^(Ninja|.*Makefiles)$")
        message(STATUS "Reflection sidecars are not generated with ${CMAKE_GENERATOR}: it writes no compile_commands.json")
    else()
        list(GET USER_SOURCES 0 _scan_unit)
        set(_scan_flags "")
        if(MSVC)
            list(APPEND _scan_flags --extra-arg=-Xclang --extra-arg=-std=c++23)
        else()
            list(APPEND _scan_flags --extra-arg=-Wno-ignored-gch)
        endif()

        set(_sidecars "")

        if(MAYAFLUX_DESCRIBED_HEADERS)
            find_program(MAYAFLUX_REFLECT_EXECUTABLE mayaflux_reflect HINTS "${MAYAFLUX_PREFIX}/bin")
            if(NOT MAYAFLUX_REFLECT_EXECUTABLE)
                message(FATAL_ERROR "mayaflux_reflect not found in ${MAYAFLUX_PREFIX}/bin. Reinstall MayaFlux or set MAYAFLUX_ROOT.")
            endif()
            foreach(_header IN LISTS MAYAFLUX_DESCRIBED_HEADERS)
                get_filename_component(_dir "${_header}" DIRECTORY)
                get_filename_component(_stem "${_header}" NAME_WLE)
                file(RELATIVE_PATH _rel "${CMAKE_SOURCE_DIR}" "${_header}")
                set(_sidecar "${_dir}/${_stem}.describe.hpp")
                set(_depfile "${CMAKE_BINARY_DIR}/mayaflux_reflect/${_rel}.d")
                add_custom_command(
                    OUTPUT "${_sidecar}"
                    COMMAND "${MAYAFLUX_REFLECT_EXECUTABLE}" --build "${_header}"
                        -p "${CMAKE_BINARY_DIR}"
                        --translation-unit "${_scan_unit}"
                        ${_scan_flags}
                        --depfile "${_depfile}"
                        --depfile-target "${_sidecar}"
                    DEPENDS "${_header}"
                    DEPFILE "${_depfile}"
                    COMMENT "Describing ${_rel}"
                    VERBATIM
                )
                list(APPEND _sidecars "${_sidecar}")
            endforeach()
        endif()

        if(MAYAFLUX_DESCRIBED_FUNCTION_HEADERS)
            find_program(MAYAFLUX_SIGNATURES_EXECUTABLE mayaflux_signatures HINTS "${MAYAFLUX_PREFIX}/bin")
            if(NOT MAYAFLUX_SIGNATURES_EXECUTABLE)
                message(FATAL_ERROR "mayaflux_signatures not found in ${MAYAFLUX_PREFIX}/bin. Reinstall MayaFlux or set MAYAFLUX_ROOT.")
            endif()
            foreach(_header IN LISTS MAYAFLUX_DESCRIBED_FUNCTION_HEADERS)
                get_filename_component(_dir "${_header}" DIRECTORY)
                get_filename_component(_stem "${_header}" NAME_WLE)
                file(RELATIVE_PATH _rel "${CMAKE_SOURCE_DIR}" "${_header}")
                set(_sidecar "${_dir}/${_stem}.signatures.hpp")
                set(_depfile "${CMAKE_BINARY_DIR}/mayaflux_reflect/${_rel}.signatures.d")
                add_custom_command(
                    OUTPUT "${_sidecar}"
                    COMMAND "${MAYAFLUX_SIGNATURES_EXECUTABLE}" --functions --build "${_header}"
                        -p "${CMAKE_BINARY_DIR}"
                        --translation-unit "${_scan_unit}"
                        ${_scan_flags}
                        --depfile "${_depfile}"
                        --depfile-target "${_sidecar}"
                    DEPENDS "${_header}"
                    DEPFILE "${_depfile}"
                    COMMENT "Describing functions in ${_rel}"
                    VERBATIM
                )
                list(APPEND _sidecars "${_sidecar}")
            endforeach()
        endif()

        target_sources(${PROJECT_NAME} PRIVATE ${_sidecars})
    endif()
endif()

# ============================================================================
# Shaders
# ============================================================================
include(cmake/shaders)

target_compile_definitions(${PROJECT_NAME} PRIVATE
    MAYAFLUX_PROJECT_SHADER_DIR="${CMAKE_SOURCE_DIR}/data/shaders"
    MAYAFLUX_PROJECT_ROOT="${CMAKE_SOURCE_DIR}"
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
