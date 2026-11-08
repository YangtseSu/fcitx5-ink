find_package(Fcitx5Core 5.0.20 REQUIRED)
find_package(nlohmann_json 3.11 REQUIRED)
find_path(TOMLPLUSPLUS_INCLUDE_DIR NAMES toml++/toml.hpp REQUIRED)

add_library(fcitx5-ink MODULE ink.cpp config.cpp)
target_compile_features(fcitx5-ink PRIVATE cxx_std_20)
target_compile_options(fcitx5-ink PRIVATE -Wall -Wextra -Werror)
target_include_directories(fcitx5-ink PRIVATE "${TOMLPLUSPLUS_INCLUDE_DIR}")
target_compile_definitions(fcitx5-ink PRIVATE
  FCITX5_INK_PANEL_PATH="${CMAKE_INSTALL_FULL_BINDIR}/fcitx5-ink-panel")
target_link_libraries(fcitx5-ink PRIVATE
  Fcitx5::Core
  nlohmann_json::nlohmann_json)

install(TARGETS fcitx5-ink LIBRARY DESTINATION "${FCITX5_ADDON_DIR}")

# Fcitx5 appends the platform suffix to Library; leave .so out.
set(INK_INSTALL_LIBRARY "${CMAKE_INSTALL_FULL_LIBDIR}/fcitx5/libfcitx5-ink")
configure_file(ink.conf.in "${CMAKE_CURRENT_BINARY_DIR}/install/ink.conf" @ONLY)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/install/ink.conf"
        DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/addon"
        RENAME ink.conf)

if(BUILD_TESTING)
  add_executable(fcitx5-ink-config-test
    "${CMAKE_SOURCE_DIR}/tests/config-test.cpp" config.cpp)
  target_compile_features(fcitx5-ink-config-test PRIVATE cxx_std_20)
  target_compile_options(fcitx5-ink-config-test PRIVATE -Wall -Wextra -Werror)
  target_include_directories(fcitx5-ink-config-test PRIVATE
    "${TOMLPLUSPLUS_INCLUDE_DIR}")
  target_link_libraries(fcitx5-ink-config-test PRIVATE Fcitx5::Utils)
  add_test(NAME fcitx5-ink-config COMMAND fcitx5-ink-config-test)
endif()

# Run-from-build-tree layout for the isolated tests. There is no input method
# entry: the addon is a module and the hotkey is the only entry point.
set(INK_TEST_DATA_DIR "${CMAKE_CURRENT_BINARY_DIR}/test-data")
# Remove a manifest left by an earlier build: the addon is no longer an input
# method and the isolated tests must not see a stale entry.
file(REMOVE_RECURSE "${INK_TEST_DATA_DIR}/fcitx5/inputmethod")
file(MAKE_DIRECTORY "${INK_TEST_DATA_DIR}/fcitx5/addon")
file(GENERATE
  OUTPUT "${INK_TEST_DATA_DIR}/fcitx5/addon/ink.conf"
  CONTENT "[Addon]
Name=Ink
Name[zh_CN]=墨笔
Category=Module
Type=SharedLibrary
Library=$<TARGET_FILE_DIR:fcitx5-ink>/libfcitx5-ink
OnDemand=False
Configurable=False

[Addon/Dependencies]
0=core:5.0.20
")
