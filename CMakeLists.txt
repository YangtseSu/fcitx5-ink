cmake_minimum_required(VERSION 3.25)
project(fcitx5-ink VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)
include(CTest)

set(FCITX5_ADDON_DIR "${CMAKE_INSTALL_LIBDIR}/fcitx5" CACHE PATH "Fcitx5 addon directory")

add_subdirectory(addon)

set(FCITX5_INK_PANEL_BINARY "" CACHE FILEPATH "Prebuilt fcitx5-ink-panel binary")
set(FCITX5_INK_MODEL_DIR "" CACHE PATH "Directory with handwriting-*.model files to install")
if(FCITX5_INK_PANEL_BINARY)
  install(PROGRAMS "${FCITX5_INK_PANEL_BINARY}"
          DESTINATION "${CMAKE_INSTALL_BINDIR}")
endif()
if(FCITX5_INK_MODEL_DIR)
  file(GLOB INK_MODELS "${FCITX5_INK_MODEL_DIR}/handwriting-*.model")
  if(INK_MODELS)
    install(FILES ${INK_MODELS}
            DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5-ink")
  endif()
  if(EXISTS "${FCITX5_INK_MODEL_DIR}/HandwritingModel-LICENSE.txt")
    install(FILES "${FCITX5_INK_MODEL_DIR}/HandwritingModel-LICENSE.txt"
            DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5-ink")
  endif()
endif()
install(FILES
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE"
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSES/MPL-2.0.txt"
          "${CMAKE_CURRENT_SOURCE_DIR}/LICENSES/BSD-3-Clause.txt"
        DESTINATION "${CMAKE_INSTALL_DATADIR}/licenses/fcitx5-ink")
