set(MF_MIN_VERSION "0.4.0")
set(MF_NEEDS_LILA OFF)

file(GLOB_RECURSE _@MODULE_NAME@_sources CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/community/@MODULE_NAME@/src/*.cpp"
)

add_library(@MODULE_NAME@ OBJECT
    ${_@MODULE_NAME@_sources}
)

target_include_directories(@MODULE_NAME@ PUBLIC
    "${CMAKE_SOURCE_DIR}/community/@MODULE_NAME@/src"
)

set_target_properties(@MODULE_NAME@ PROPERTIES
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
)

target_link_libraries(@MODULE_NAME@ PUBLIC MayaFlux::MayaFluxLib)

if(MF_NEEDS_LILA)
    target_link_libraries(@MODULE_NAME@ PUBLIC MayaFlux::MayaFluxHost)
endif()
