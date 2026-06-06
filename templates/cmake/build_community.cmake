# build_community.cmake
# Provides add_community() for linking MayaFlux community modules.
#
# Usage in community.cmake:
#   add_community(module_name)
#
# Each module lives in ${CMAKE_SOURCE_DIR}/community/<name>/
# and must provide <name>.cmake and src/.
#
# Modules are acquired via: weave community add <name>

function(add_community name)

    set(_mod_dir "${CMAKE_SOURCE_DIR}/community/${name}")
    set(_mod_cmake "${_mod_dir}/${name}.cmake")

    if(NOT EXISTS "${_mod_cmake}")
        message(FATAL_ERROR
            "Community module '${name}' not found.\n"
            "Expected: ${_mod_cmake}\n"
            "Run: weave community add ${name}"
        )
    endif()

    # Build the community target against MayaFlux before linking to user project
    include("${_mod_cmake}")

    if(DEFINED MF_MIN_VERSION)
        if(MayaFlux_VERSION VERSION_LESS MF_MIN_VERSION)
            message(FATAL_ERROR
                "Community module '${name}' requires MayaFlux >= ${MF_MIN_VERSION}, "
                "found ${MayaFlux_VERSION}"
            )
        endif()
        unset(MF_MIN_VERSION)
    endif()

    if(DEFINED MF_NEEDS_LILA)
        unset(MF_NEEDS_LILA)
    endif()

    if(NOT TARGET ${name})
        message(FATAL_ERROR
            "Community module '${name}' did not define target '${name}'. "
            "The module's .cmake file is malformed."
        )
    endif()

    target_precompile_headers(${name} REUSE_FROM ${PROJECT_NAME})
    target_link_libraries(${PROJECT_NAME} PRIVATE ${name})
    message(STATUS "Community module linked: ${name}")
endfunction()

# ============================================================================
# Read community.cmake and call add_community for each listed module
# ============================================================================
if(EXISTS "${CMAKE_SOURCE_DIR}/community.cmake")
    file(STRINGS "${CMAKE_SOURCE_DIR}/community.cmake" _community_modules)
    foreach(_mod ${_community_modules})
        string(STRIP "${_mod}" _mod)
        if(NOT _mod STREQUAL "" AND NOT _mod MATCHES "^#")
            add_community(${_mod})
        endif()
    endforeach()
else()
    message(STATUS "No community.cmake found, skipping community modules")
endif()
