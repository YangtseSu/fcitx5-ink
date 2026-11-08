find_package(Fcitx5Core 5.0.20 REQUIRED)
find_package(nlohmann_json 3.11 REQUIRED)

add_library(fcitx5-ink MODULE ink.cpp)
target_compile_features(fcitx5-ink PRIVATE cxx_std_20)
target_compile_options(fcitx5-ink PRIVATE -Wall -Wextra -Werror)
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
install(FILES ink-inputmethod.conf
        DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/inputmethod"
        RENAME ink.conf)

# Run-from-build-tree layout for the isolated test.
set(INK_TEST_DATA_DIR "${CMAKE_CURRENT_BINARY_DIR}/test-data")
file(MAKE_DIRECTORY "${INK_TEST_DATA_DIR}/fcitx5/addon"
                    "${INK_TEST_DATA_DIR}/fcitx5/inputmethod")
file(GENERATE
  OUTPUT "${INK_TEST_DATA_DIR}/fcitx5/addon/ink.conf"
  CONTENT "[Addon]
Name=Ink
Name[zh_CN]=墨笔
Category=InputMethod
Type=SharedLibrary
Library=$<TARGET_FILE_DIR:fcitx5-ink>/libfcitx5-ink
OnDemand=False
Configurable=False

[Addon/Dependencies]
0=core:5.0.20
")
file(GENERATE
  OUTPUT "${INK_TEST_DATA_DIR}/fcitx5/inputmethod/ink.conf"
  CONTENT "[InputMethod]
Name=Ink
Name[zh_CN]=墨笔
Label=墨
LangCode=zh_CN
Addon=ink
Configurable=False
")
