cmake_minimum_required(VERSION 3.25)
project(fcitx5-ink VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)

set(FCITX5_ADDON_DIR "${CMAKE_INSTALL_LIBDIR}/fcitx5" CACHE PATH "Fcitx5 addon directory")

add_subdirectory(addon)

set(FCITX5_INK_PANEL_BINARY "" CACHE FILEPATH "Prebuilt fcitx5-ink-panel binary")
set(FCITX5_INK_MODEL "" CACHE FILEPATH "handwriting-zh_CN.model to install")
if(FCITX5_INK_PANEL_BINARY)
  install(PROGRAMS "${FCITX5_INK_PANEL_BINARY}"
          DESTINATION "${CMAKE_INSTALL_BINDIR}")
endif()
if(FCITX5_INK_MODEL)
  install(FILES "${FCITX5_INK_MODEL}"
          DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5-ink")
endif()
install(FILES
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE"
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSES/MPL-2.0.txt"
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSES/BSD-3-Clause.txt"
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSES/LGPL-2.1-or-later.txt"
        DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/fcitx5-ink")
